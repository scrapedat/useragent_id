use anyhow::{Context, Result};
use shared::types::RecordedEvent;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use uuid::Uuid;

/// Manages the persistence of recorded events to a JSONL file.
pub struct EventLogger {
    writer: BufWriter<File>,
}

impl EventLogger {
    /// Creates a new logger for a given session ID, saving to the specified directory.
    /// The file will be named `session_<session_id>.jsonl`.
    pub fn new(session_id: Uuid, event_log_dir: &Path) -> Result<Self> {
        let file_path = event_log_dir.join(format!("session_{}.jsonl", session_id));
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&file_path)
            .with_context(|| format!("Failed to open or create event log file at {:?}", file_path))?;

        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Logs a single `RecordedEvent` to the file.
    /// Each event is serialized to a single line of JSON.
    pub fn log_event(&mut self, event: &RecordedEvent) -> Result<()> {
        let json_data = serde_json::to_string(event)
            .context("Failed to serialize event to JSON")?;
        
        writeln!(self.writer, "{}", json_data)
            .context("Failed to write event to log file")?;
        
        Ok(())
    }

    /// Flushes the buffer to ensure all events are written to disk.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().context("Failed to flush event log writer")
    }
}
