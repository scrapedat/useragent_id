use anyhow::Result;
use crossbeam_channel::Sender;
use notify::{RecommendedWatcher, Watcher};
use shared::types::RecordedEvent;
use std::path::PathBuf;

use std::thread;
use std::time::Duration;

/// Watches a specific file for changes and sends narration events.
pub struct NarrationWatcher {
    _watcher: RecommendedWatcher,
}

impl NarrationWatcher {
    /// Creates a new NarrationWatcher and starts watching the specified file.
    pub fn new(file_to_watch: PathBuf, event_sender: Sender<RecordedEvent>, session_id: uuid::Uuid) -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;

        watcher.watch(&file_to_watch, notify::RecursiveMode::NonRecursive)?;

        // Spawn a thread to process file change events
        thread::spawn(move || {
            let mut last_content = String::new();
            loop {
                // Check for new events every second
                thread::sleep(Duration::from_secs(1));

                if rx.try_recv().is_ok() {
                    if let Ok(current_content) = std::fs::read_to_string(&file_to_watch) {
                        let new_text = current_content.trim().to_string();
                        if !new_text.is_empty() && new_text != last_content {
                            let event = RecordedEvent {
                                session_id,
                                timestamp: chrono::Utc::now(),
                                event_type: shared::types::EventType::Narration(new_text.clone()),
                            };
                            if event_sender.send(event).is_err() {
                                // Stop if the channel is closed
                                break;
                            }
                            last_content = new_text;
                        }
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
        })
    }
}
