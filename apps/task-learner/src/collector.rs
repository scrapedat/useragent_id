use anyhow::{Context, Result};
use shared::core::config::AppConfig;
use shared::types::RecordedEvent;
use std::fs;
use std::path::PathBuf;
use glob::glob;
use std::io::BufRead;

/// Finds the most recent session log file in the event log directory.
fn find_latest_session_file(config: &AppConfig) -> Result<Option<PathBuf>> {
    let pattern = config.event_log_dir.join("session_*.jsonl");
    let mut paths: Vec<PathBuf> = Vec::new();

    for entry in glob(pattern.to_str().unwrap_or_default())? {
        if let Ok(path) = entry {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        return Ok(None);
    }

    // Find the file with the latest modification time
    let latest_path = paths.into_iter().max_by_key(|path| {
        fs::metadata(path).ok().and_then(|m| m.modified().ok())
    });

    Ok(latest_path)
}

/// Loads all recorded events from the most recent session log file.
pub fn load_latest_session_events(config: &AppConfig) -> Result<Vec<RecordedEvent>> {
    let latest_file = find_latest_session_file(config)?
        .context("No session log files found.")?;

    println!("Loading events from: {}", latest_file.display());

    let file = fs::File::open(latest_file)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: RecordedEvent = serde_json::from_str(&line)
            .with_context(|| format!("Failed to deserialize event from line: {}", line))?;
        events.push(event);
    }

    // Optional: append DOM events (raw lines) to the end as placeholders, or keep separate.
    // Here we keep OS events only and rely on the learner to merge later if needed.
    Ok(events)
}

/// Load tail of a corresponding dom_session_<session_id>.jsonl if present.
pub fn load_dom_events_for_session(config: &AppConfig, session_id: &uuid::Uuid) -> Result<Vec<String>> {
    let dom_path = config.event_log_dir.join(format!("dom_session_{}.jsonl", session_id));
    if !dom_path.exists() { return Ok(vec![]); }
    let f = fs::File::open(dom_path)?;
    let rdr = std::io::BufReader::new(f);
    let mut lines = Vec::new();
    for line in rdr.lines() { lines.push(line?); }
    Ok(lines)
}
