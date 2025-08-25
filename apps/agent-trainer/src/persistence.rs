use anyhow::{Context, Result};
use shared::types::Agent;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const TRAINED_AGENTS_DIR: &str = "trained-agents";

/// Saves a trained agent's metadata to the `trained-agents` directory.
pub fn save_agent_metadata(agent: &Agent) -> Result<PathBuf> {
    let dir = Path::new(TRAINED_AGENTS_DIR);
    if !dir.exists() {
        fs::create_dir_all(dir).context("Failed to create trained-agents directory")?;
    }

    // Save the agent metadata
    let metadata_filename = format!("{}.json", agent.id);
    let metadata_path = dir.join(metadata_filename);
    let metadata_file = File::create(&metadata_path)
        .with_context(|| format!("Failed to create agent metadata file at {:?}", metadata_path))?;
    serde_json::to_writer_pretty(metadata_file, agent)
        .context("Failed to serialize agent metadata")?;

    Ok(metadata_path)
}

/// Saves the generated code for an agent.
pub fn save_agent_code(agent_id: &uuid::Uuid, code: &str) -> Result<PathBuf> {
    let dir = Path::new(TRAINED_AGENTS_DIR);
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    let code_filename = format!("{}.rs", agent_id);
    let code_path = dir.join(&code_filename);
    let mut file = File::create(&code_path)?;
    file.write_all(code.as_bytes())?;
    Ok(code_path)
}
