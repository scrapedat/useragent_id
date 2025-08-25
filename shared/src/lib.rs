pub mod types;
pub mod core;

use std::path::PathBuf;

// Directory constants
pub const DATA_DIR: &str = "data";
pub const RECORDINGS_DIR: &str = "data/recordings";
pub const MODELS_DIR: &str = "data/models";
pub const AGENTS_DIR: &str = "data/agents";
pub const TEMP_DIR: &str = "data/temp";

// Helper functions for data management
pub fn get_app_data_dir(app_id: &str) -> PathBuf {
    PathBuf::from(DATA_DIR).join(app_id)
}

pub fn cleanup_temp_files() -> std::io::Result<()> {
    let temp_dir = PathBuf::from(TEMP_DIR);
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
        std::fs::create_dir(&temp_dir)?;
    }
    Ok(())
}

pub fn init_data_dirs() -> std::io::Result<()> {
    for dir in [DATA_DIR, RECORDINGS_DIR, MODELS_DIR, AGENTS_DIR, TEMP_DIR] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

// Resource management helpers
pub fn check_disk_space(path: &PathBuf) -> std::io::Result<u64> {
    if let Ok(metadata) = std::fs::metadata(path) {
        Ok(metadata.len())
    } else {
        Ok(0)
    }
}

// Error handling
#[derive(Debug, thiserror::Error)]
pub enum SharedError {
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Data format error: {0}")]
    DataFormatError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}