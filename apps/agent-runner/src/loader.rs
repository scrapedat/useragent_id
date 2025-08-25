use anyhow::{Context, Result};
use shared::types::Agent;
use std::fs;
use std::path::PathBuf;

const TRAINED_AGENTS_DIR: &str = "trained-agents";

/// Finds and loads all agent metadata files from the `trained-agents` directory.
pub fn load_all_agents() -> Result<Vec<Agent>> {
    let dir_path = PathBuf::from(TRAINED_AGENTS_DIR);
    if !dir_path.exists() {
        // If the directory doesn't exist, it's not an error, just return empty.
        return Ok(Vec::new());
    }

    let mut agents = Vec::new();
    for entry in fs::read_dir(dir_path).context("Failed to read trained-agents directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // We are looking for the metadata files, which are .json
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file_content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read agent file: {:?}", path))?;
            let agent: Agent = serde_json::from_str(&file_content)
                .with_context(|| format!("Failed to deserialize agent from: {:?}", path))?;
            agents.push(agent);
        }
    }

    Ok(agents)
}
