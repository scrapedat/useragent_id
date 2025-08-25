use shared::types::{RecordedEvent, LearnedTask, AutomationStep, ActionType, ElementIdentifier};
use std::collections::HashMap;
use uuid::Uuid;

const MIN_PATTERN_LENGTH: usize = 2;
const MIN_PATTERN_FREQUENCY: usize = 2;

/// Analyzes a stream of recorded events to find repetitive patterns.
pub fn learn_task_from_events(events: &[RecordedEvent]) -> Option<LearnedTask> {
    if events.len() < MIN_PATTERN_LENGTH {
        return None;
    }

    let simplified_events: Vec<String> = events.iter().map(simplify_event).collect();
    let mut sequence_counts: HashMap<&[String], usize> = HashMap::new();

    for window in simplified_events.windows(MIN_PATTERN_LENGTH) {
        *sequence_counts.entry(window).or_insert(0) += 1;
    }

    let most_frequent_sequence = sequence_counts
        .into_iter()
        .filter(|(_, count)| *count >= MIN_PATTERN_FREQUENCY)
        .max_by_key(|(_, count)| *count);

    if let Some((sequence, _)) = most_frequent_sequence {
        let steps = sequence.iter().map(|event_str| {
            AutomationStep {
                action_type: ActionType::Click, // Placeholder
                target: Some(ElementIdentifier {
                    using: "css".to_string(),
                    value: event_str.to_string(), // Placeholder
                }),
                data: None,
                description: format!("Perform action: {}", event_str),
            }
        }).collect();

        return Some(LearnedTask {
            id: Uuid::new_v4(),
            name: "New Learned Task".to_string(),
            description: format!("A task learned from a sequence of {} actions.", sequence.len()),
            steps,
            source_session_id: events.first().map_or(Uuid::nil(), |e| e.session_id),
        });
    }

    None
}

/// Simplifies an event into a string representation for pattern matching.
fn simplify_event(event: &RecordedEvent) -> String {
    match &event.event_type {
        shared::types::EventType::KeyPress(key) => format!("KeyPress({:?})", key),
        shared::types::EventType::KeyRelease(key) => format!("KeyRelease({:?})", key),
        shared::types::EventType::ButtonPress(btn) => format!("ButtonPress({:?})", btn),
        shared::types::EventType::ButtonRelease(btn) => format!("ButtonRelease({:?})", btn),
        shared::types::EventType::MouseMove { x, y } => format!("MouseMove({},{})", x.round(), y.round()),
        shared::types::EventType::Narration(text) => format!("Narration: {}", text),
    }
}

/// New entry point that can use DOM JSONL lines to bias/simplify events.
pub fn learn_task_from_events_with_dom(events: &[RecordedEvent], dom_lines: &[String]) -> Option<LearnedTask> {
    // 1) Prefer DOM-derived concrete steps if any DOM events exist.
    if !dom_lines.is_empty() {
        let dom_events = parse_dom_lines(dom_lines);
        if !dom_events.is_empty() {
            let steps = build_steps_from_dom(dom_events);
            if !steps.is_empty() {
                return Some(LearnedTask {
                    id: Uuid::new_v4(),
                    name: "DOM-derived Task".to_string(),
                    description: format!("Task generated from {} DOM events.", steps.len()),
                    steps,
                    source_session_id: events.first().map_or(Uuid::nil(), |e| e.session_id),
                });
            }
        }
    }

    // 2) Fallback to simple pattern mining over OS events with light DOM context.
    if events.len() < MIN_PATTERN_LENGTH { return None; }
    let dom_clues = DomClues::from_lines(dom_lines);
    let simplified_events: Vec<String> = events.iter().map(|e| simplify_event_with_dom(e, &dom_clues)).collect();
    let mut sequence_counts: HashMap<&[String], usize> = HashMap::new();
    for window in simplified_events.windows(MIN_PATTERN_LENGTH) {
        *sequence_counts.entry(window).or_insert(0) += 1;
    }
    let most_frequent_sequence = sequence_counts
        .into_iter()
        .filter(|(_, count)| *count >= MIN_PATTERN_FREQUENCY)
        .max_by_key(|(_, count)| *count);
    if let Some((sequence, _)) = most_frequent_sequence {
        let steps = sequence.iter().map(|event_str| AutomationStep {
            action_type: ActionType::Click,
            target: Some(ElementIdentifier { using: "css".to_string(), value: event_str.to_string() }),
            data: None,
            description: format!("Perform action: {}", event_str),
        }).collect();
        return Some(LearnedTask {
            id: Uuid::new_v4(),
            name: "New Learned Task".to_string(),
            description: format!("A task learned from a sequence of {} actions.", sequence.len()),
            steps,
            source_session_id: events.first().map_or(Uuid::nil(), |e| e.session_id),
        });
    }
    None
}

struct DomClues {
    last_url: Option<String>,
    last_title: Option<String>,
}

impl DomClues {
    fn from_lines(lines: &[String]) -> Self {
        let mut clues = DomClues { last_url: None, last_title: None };
        for line in lines {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
                if t == "navigated" {
                    if let Some(url) = v.get("payload").and_then(|p| p.get("url")).and_then(|u| u.as_str()) {
                        clues.last_url = Some(url.to_string());
                    }
                    if let Some(title) = v.get("payload").and_then(|p| p.get("title")).and_then(|u| u.as_str()) {
                        clues.last_title = Some(title.to_string());
                    }
                } else if t == "page_load" {
                    if let Some(title) = v.get("payload").and_then(|p| p.get("title")).and_then(|u| u.as_str()) {
                        clues.last_title = Some(title.to_string());
                    }
                }
            }
        }
        clues
    }
}

fn simplify_event_with_dom(event: &RecordedEvent, clues: &DomClues) -> String {
    match &event.event_type {
        shared::types::EventType::Narration(text) => {
            if let Some(title) = &clues.last_title {
                format!("Narration@{}: {}", title, text)
            } else if let Some(url) = &clues.last_url {
                format!("Narration@{}: {}", url, text)
            } else {
                format!("Narration: {}", text)
            }
        }
        shared::types::EventType::KeyPress(key) => format!("KeyPress({:?})", key),
        shared::types::EventType::KeyRelease(key) => format!("KeyRelease({:?})", key),
        shared::types::EventType::ButtonPress(btn) => format!("ButtonPress({:?})", btn),
        shared::types::EventType::ButtonRelease(btn) => format!("ButtonRelease({:?})", btn),
        shared::types::EventType::MouseMove { x, y } => format!("MouseMove({},{})", x.round(), y.round()),
    }
}

// ===== DOM parsing and step construction =====

#[derive(Debug, Clone)]
struct DomEvent {
    timestamp: u64,
    kind: DomEventKind,
}

#[derive(Debug, Clone)]
enum DomEventKind {
    Navigated { url: Option<String>, title: Option<String> },
    PageLoad { title: Option<String> },
    Click { xpath: Option<String>, css: Option<String>, tag: Option<String>, text: Option<String> },
    KeyDown { key: Option<String> },
    Error { message: String },
}

fn parse_dom_lines(lines: &[String]) -> Vec<DomEvent> {
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() { continue; }
        let v = match serde_json::from_str::<serde_json::Value>(line) { Ok(v) => v, Err(_) => continue };
        let ts = idx as u64; // Preserve file order as event time
        let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let kind = match t {
            "navigated" => {
                let p = v.get("payload").cloned().unwrap_or_default();
                let url = p.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
                let title = p.get("title").and_then(|u| u.as_str()).map(|s| s.to_string());
                DomEventKind::Navigated { url, title }
            }
            "page_load" => {
                let p = v.get("payload").cloned().unwrap_or_default();
                let title = p.get("title").and_then(|u| u.as_str()).map(|s| s.to_string());
                DomEventKind::PageLoad { title }
            }
            "dom_event" => {
                let p = v.get("payload").cloned().unwrap_or_default();
                match p.get("kind").and_then(|k| k.as_str()) {
                    Some("click") => DomEventKind::Click {
                        xpath: p.get("xpath").and_then(|x| x.as_str()).map(|s| s.to_string()),
                        css: p.get("css").and_then(|x| x.as_str()).map(|s| s.to_string()),
                        tag: p.get("tag").and_then(|x| x.as_str()).map(|s| s.to_string()),
                        text: p.get("text").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    },
                    Some("keydown") => DomEventKind::KeyDown { key: p.get("key").and_then(|x| x.as_str()).map(|s| s.to_string()) },
                    _ => continue,
                }
            }
            "error" => {
                let msg = v.get("error").and_then(|x| x.as_str()).unwrap_or("error");
                DomEventKind::Error { message: msg.to_string() }
            }
            _ => continue,
        };
        out.push(DomEvent { timestamp: ts, kind });
    }
    // Lines are already chronological; return as-is
    out
}

fn build_steps_from_dom(events: Vec<DomEvent>) -> Vec<AutomationStep> {
    let mut steps: Vec<AutomationStep> = Vec::new();
    let mut last_target: Option<ElementIdentifier> = None;
    let mut type_buffer: String = String::new();
    let mut last_was_enter: bool = false;

    let flush_typing = |steps: &mut Vec<AutomationStep>, last_target: &Option<ElementIdentifier>, buf: &mut String| {
        if !buf.is_empty() {
            steps.push(AutomationStep {
                action_type: ActionType::Type,
                target: last_target.clone(),
                data: Some(buf.clone()),
                description: match last_target {
                    Some(t) => format!("Type '{}' into {}", buf, t.value),
                    None => format!("Type '{}'", buf),
                },
            });
            buf.clear();
        }
    };

    // We'll iterate by index to peek next event cheaply
    let len = events.len();
    let mut i = 0usize;
    while i < len {
        let e = &events[i];
        match &e.kind {
            DomEventKind::Navigated { url, title } => {
                // Flush any pending typing before navigation
                flush_typing(&mut steps, &last_target, &mut type_buffer);
                if let Some(u) = url {
                    steps.push(AutomationStep {
                        action_type: ActionType::Navigate,
                        target: None,
                        data: Some(u.clone()),
                        description: match title {
                            Some(t) => format!("Navigate to {} ({})", u, t),
                            None => format!("Navigate to {}", u),
                        },
                    });
                }
            }
            DomEventKind::PageLoad { title } => {
                flush_typing(&mut steps, &last_target, &mut type_buffer);
                steps.push(AutomationStep {
                    action_type: ActionType::Wait,
                    target: None,
                    data: None,
                    description: match title { Some(t) => format!("Wait for page load: {}", t), None => "Wait for page load".to_string() },
                });
                last_was_enter = false;
            }
            DomEventKind::Click { xpath, css, tag, text } => {
                flush_typing(&mut steps, &last_target, &mut type_buffer);
                let target = if let Some(sel) = css.clone() {
                    Some(ElementIdentifier { using: "css".to_string(), value: sel })
                } else if let Some(xp) = xpath.clone() {
                    Some(ElementIdentifier { using: "xpath".to_string(), value: xp })
                } else { None };
                if let Some(t) = &target { last_target = Some(t.clone()); }
                steps.push(AutomationStep {
                    action_type: ActionType::Click,
                    target: target.clone(),
                    data: None,
                    description: match (tag, text) {
                        (Some(tag), Some(txt)) if !txt.is_empty() => format!("Click {} '{}'", tag, txt),
                        (Some(tag), _) => format!("Click {}", tag),
                        _ => match &target { Some(t) => format!("Click {}", t.value), None => "Click".to_string() },
                    },
                });
                last_was_enter = false;
            }
            DomEventKind::KeyDown { key } => {
                if let Some(k) = key.as_deref() {
                    match k {
                        "Enter" => {
                            flush_typing(&mut steps, &last_target, &mut type_buffer);
                            // Tentatively mark as Enter; if a navigation follows next, convert to Execute.
                            steps.push(AutomationStep { action_type: ActionType::Wait, target: None, data: None, description: "Enter pressed".to_string() });
                            last_was_enter = true;
                        }
                        "Backspace" => { type_buffer.pop(); }
                        "Tab" => {
                            flush_typing(&mut steps, &last_target, &mut type_buffer);
                            last_was_enter = false;
                        }
                        " " => { type_buffer.push(' '); }
                        k if k.len() == 1 => { type_buffer.push_str(k); }
                        _ => { /* ignore other control keys for now */ }
                    }
                }
            }
            DomEventKind::Error { .. } => { /* ignore in step generation, could be surfaced elsewhere */ }
        }
        // If Enter was pressed and the next immediate event is a navigation or page load, convert to Execute
        if last_was_enter {
            if let Some(next) = events.get(i + 1) {
                match next.kind {
                    DomEventKind::Navigated { .. } | DomEventKind::PageLoad { .. } => {
                        if let Some(last) = steps.last_mut() {
                            last.action_type = ActionType::Execute;
                            last.description = "Submit/Enter (navigation followed)".to_string();
                        }
                        last_was_enter = false;
                    }
                    _ => {}
                }
            }
        }
        i += 1;
    }

    // Flush trailing typing at the end
    if !type_buffer.is_empty() {
        steps.push(AutomationStep {
            action_type: ActionType::Type,
            target: last_target.clone(),
            data: Some(type_buffer.clone()),
            description: match &last_target { Some(t) => format!("Type '{}' into {}", type_buffer, t.value), None => format!("Type '{}'", type_buffer) },
        });
    }

    steps
}
