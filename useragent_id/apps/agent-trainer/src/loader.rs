use anyhow::{Context, Result};
use shared::types::LearnedTask;
use std::fs;
use std::path::PathBuf;

const LEARNED_TASKS_DIR: &str = "../learned_tasks";

/// Finds and loads the most recent learned task from the `learned_tasks` directory.
pub fn load_latest_learned_task() -> Result<LearnedTask> {
    let learned_tasks_dir = PathBuf::from(LEARNED_TASKS_DIR);
    if !learned_tasks_dir.exists() {
        return Err(anyhow::anyhow!("Learned tasks directory not found at {:?}", learned_tasks_dir.canonicalize().unwrap_or_default()));
    }

    // Find the most recently modified file in the directory.
    let latest_entry = fs::read_dir(learned_tasks_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap());

    if let Some(entry) = latest_entry {
        let path = entry.path();
        let json_data = fs::read_to_string(&path)
            .context(format!("Failed to read learned task file: {:?}", path))?;
        let task: LearnedTask = serde_json::from_str(&json_data)
            .context("Failed to deserialize learned task from JSON")?;
        Ok(task)
    } else {
        Err(anyhow::anyhow!("No learned tasks found in the directory"))
    }
}
