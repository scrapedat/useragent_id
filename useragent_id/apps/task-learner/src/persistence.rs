use anyhow::{Context, Result};
use shared::core::config::AppConfig;
use shared::types::LearnedTask;
use std::fs;
use std::io::Write;



/// Saves a `LearnedTask` to a file in the `learned_tasks` directory.
/// The file will be named `learned_task_<uuid>.json`.
pub fn save_learned_task(task: &LearnedTask, config: &AppConfig) -> Result<()> {
    let dir = &config.learned_tasks_dir;
    if !dir.exists() {
        fs::create_dir_all(dir).context("Failed to create learned_tasks directory")?;
    }

    let file_path = dir.join(format!("task_{}.json", task.id));
    println!("Saving learned task to: {}", file_path.display());

    let mut file = fs::File::create(&file_path)
        .context(format!("Failed to create file: {:?}", file_path))?;
    let json = serde_json::to_string_pretty(task)
        .context("Failed to serialize learned task to JSON")?;
    file.write_all(json.as_bytes())
        .context("Failed to write learned task to file")?;

    println!("Task saved successfully.");
    Ok(())
}
