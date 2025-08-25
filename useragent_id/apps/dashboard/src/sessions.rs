use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::core::config::AppConfig;
use shared::types::{EventType, RecordedEvent};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub log_path: PathBuf,
    pub num_events: usize,
    pub num_narrations: usize,
    pub num_dom_events: usize,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub title: String,
    pub description: String,
}

fn meta_path_for(log_path: &Path) -> PathBuf {
    let stem = log_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let meta_name = format!("{}.meta.json", stem);
    log_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(meta_name)
}

fn default_title(session_id: &Uuid) -> String {
    let short = &session_id.to_string()[..8];
    format!("Session {}", short)
}

/// Read the last `max_lines` of a text file efficiently.
pub fn tail_lines(path: &Path, max_lines: usize) -> Result<Vec<String>> {
    if !path.exists() { return Ok(vec![]); }
    let file = File::open(path).with_context(|| format!("Open {:?}", path))?;
    let reader = BufReader::new(file);
    let mut ring: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(max_lines);
    for line in reader.lines() {
        let line = line.unwrap_or_default();
        if ring.len() == max_lines { ring.pop_front(); }
        ring.push_back(line);
    }
    Ok(ring.into_iter().collect())
}

/// Read the last `max` narration lines from a session JSONL log.
pub fn tail_narrations(path: &Path, max: usize) -> Result<Vec<String>> {
    if !path.exists() { return Ok(vec![]); }
    let file = File::open(path).with_context(|| format!("Open {:?}", path))?;
    let reader = BufReader::new(file);
    let mut narrs: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Ok(ev) = serde_json::from_str::<RecordedEvent>(&line) {
            if let EventType::Narration(text) = ev.event_type { narrs.push(text); }
        }
    }
    let len = narrs.len();
    let start = len.saturating_sub(max);
    Ok(narrs[start..].to_vec())
}

/// Extract the latest URL/title from a DOM JSONL log based on common keys.
pub fn last_dom_location(path: &Path) -> Result<Option<(String, Option<String>)>> {
    if !path.exists() { return Ok(None); }
    let file = File::open(path)?;
    let rdr = BufReader::new(file);
    let mut last_url: Option<String> = None;
    let mut last_title: Option<String> = None;
    for line in rdr.lines() {
        let line = line?;
        // Try to parse minimal structures we emit from dom_recorder.js
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
            let t = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "navigated" {
                if let Some(url) = val.get("payload").and_then(|p| p.get("url")).and_then(|u| u.as_str()) {
                    last_url = Some(url.to_string());
                }
            }
            if t == "page_load" {
                // sometimes Page.getNavigationHistory could be added later; for now skip
            }
            if t == "dom_event" {
                // no URL here; ignore
            }
            if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
                last_url = Some(url.to_string());
            }
            if let Some(title) = val.get("title").and_then(|u| u.as_str()) {
                last_title = Some(title.to_string());
            }
        }
    }
    Ok(last_url.map(|u| (u, last_title)))
}

pub fn load_sessions(config: &AppConfig) -> Result<Vec<SessionSummary>> {
    let mut out: Vec<SessionSummary> = Vec::new();
    let dir = &config.event_log_dir;
    if !dir.exists() {
        return Ok(out);
    }

    for entry in fs::read_dir(dir).with_context(|| format!("Reading dir {:?}", dir))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("session_"))
                .unwrap_or(false)
        {
            if let Some(sum) = summarize_session(&path)? {
                out.push(sum);
            }
        }
    }

    // Sort by start desc
    out.sort_by_key(|s| std::cmp::Reverse(s.start));
    Ok(out)
}

fn summarize_session(path: &Path) -> Result<Option<SessionSummary>> {
    let file =
        File::open(path).with_context(|| format!("Open session log {:?}", path))?;
    let reader = BufReader::new(file);

    let mut num_events = 0usize;
    let mut num_narr = 0usize;
    let mut start: Option<DateTime<Utc>> = None;
    let mut end: Option<DateTime<Utc>> = None;
    let mut session_id: Option<Uuid> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<RecordedEvent>(&line) {
            num_events += 1;
            if matches!(ev.event_type, EventType::Narration(_)) {
                num_narr += 1;
            }
            start = Some(start.map_or(ev.timestamp, |s| s.min(ev.timestamp)));
            end = Some(end.map_or(ev.timestamp, |e| e.max(ev.timestamp)));
            session_id = Some(ev.session_id);
        }
    }

    let session_id = match session_id {
        Some(id) => id,
        None => return Ok(None),
    };
    let meta_path = meta_path_for(path);
    let (title, description) = match load_meta(&meta_path) {
        Ok(Some(m)) => (m.title, m.description),
        _ => (default_title(&session_id), String::new()),
    };

    // Count dom_session_* for same session id
    let mut num_dom_events = 0usize;
    if let Some(dir) = path.parent() {
        let dom_name = format!("dom_session_{}.jsonl", session_id);
        let dom_path = dir.join(dom_name);
        if dom_path.exists() {
            let f = File::open(&dom_path)?;
            let r = BufReader::new(f);
            for line in r.lines() {
                let _ = line?;
                num_dom_events += 1;
            }
        }
    }

    Ok(Some(SessionSummary {
        session_id,
        log_path: path.to_path_buf(),
        num_events,
        num_narrations: num_narr,
    num_dom_events,
        start,
        end,
        title,
        description,
    }))
}

pub fn load_meta(meta_path: &Path) -> Result<Option<SessionMeta>> {
    if !meta_path.exists() {
        return Ok(None);
    }
    let f =
        File::open(meta_path).with_context(|| format!("Open meta {:?}", meta_path))?;
    let meta: SessionMeta =
        serde_json::from_reader(f).with_context(|| "Parse meta json")?;
    Ok(Some(meta))
}

pub fn save_meta_for(summary: &SessionSummary) -> Result<()> {
    let meta_path = meta_path_for(&summary.log_path);
    save_meta(
        &meta_path,
        &SessionMeta {
            session_id: summary.session_id,
            title: summary.title.clone(),
            description: summary.description.clone(),
            created_at: summary.start.unwrap_or_else(Utc::now),
            updated_at: Utc::now(),
        },
    )
}

fn save_meta(meta_path: &Path, meta: &SessionMeta) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(meta_path)
        .with_context(|| format!("Open meta for write {:?}", meta_path))?;
    serde_json::to_writer_pretty(BufWriter::new(file), meta)
        .with_context(|| "Write meta json")?;
    Ok(())
}