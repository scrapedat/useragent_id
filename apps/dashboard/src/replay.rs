use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared::types::{EventType, RecordedEvent};

#[derive(Debug, Deserialize)]
struct DomLineMeta {
    #[serde(default)]
    timestamp: Option<String>,
}

/// Align narration texts to indices in the DOM log using timestamps.
/// Fallback to proportional interleave if timestamps are missing.
pub fn align_narration_to_dom(
    dom_lines: &[String],
    os_lines: &[RecordedEvent],
) -> Vec<(usize, String)> {
    // Collect narration events with timestamps
    let mut narrs: Vec<(DateTime<Utc>, String)> = os_lines
        .iter()
        .filter_map(|e| match &e.event_type {
            EventType::Narration(t) => Some((e.timestamp, t.clone())),
            _ => None,
        })
        .collect();

    if narrs.is_empty() || dom_lines.is_empty() {
        return Vec::new();
    }

    // Parse DOM line timestamps; keep original index
    let mut dom_ts: Vec<(usize, DateTime<Utc>)> = Vec::with_capacity(dom_lines.len());
    for (i, l) in dom_lines.iter().enumerate() {
        if let Ok(meta) = serde_json::from_str::<DomLineMeta>(l) {
            if let Some(ts_str) = meta.timestamp {
                if let Ok(dt_fixed) = DateTime::parse_from_rfc3339(&ts_str) {
                    dom_ts.push((i, dt_fixed.with_timezone(&Utc)));
                }
            }
        }
    }

    if dom_ts.len() < 2 {
        // Fallback: proportional spread if we have insufficient timestamps
        let n_dom = dom_lines.len().max(1);
        let n_narr = narrs.len();
        return narrs
            .into_iter()
            .enumerate()
            .map(|(i, (_, t))| {
                let idx = (i * n_dom) / n_narr;
                (idx.min(n_dom.saturating_sub(1)), t)
            })
            .collect();
    }

    // Ensure both sequences are sorted by time
    narrs.sort_by_key(|(ts, _)| *ts);
    dom_ts.sort_by_key(|(_, ts)| *ts);

    // Two-pointer alignment: for each narration time, find nearest dom index.
    let mut result: Vec<(usize, String)> = Vec::with_capacity(narrs.len());
    let mut j = 0usize; // pointer in dom_ts
    for (nt, text) in narrs.into_iter() {
        // advance j while next dom timestamp is closer to nt
        while j + 1 < dom_ts.len() {
            let (_, dj) = dom_ts[j];
            let (_, dj1) = dom_ts[j + 1];
            let d0 = (nt - dj).num_milliseconds().abs();
            let d1 = (nt - dj1).num_milliseconds().abs();
            if d1 <= d0 {
                j += 1;
            } else {
                break;
            }
        }
        let (idx, _) = dom_ts[j];
        result.push((idx, text));
    }

    result
}
