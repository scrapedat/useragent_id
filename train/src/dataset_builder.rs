use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;
use std::fs;

#[derive(Serialize)]
pub struct InstructionPair { pub instruction: String, pub input: String, pub output: String }

pub fn build_internal_dataset(root: &str, out_path: &str) -> Result<()> {
    let mut lines = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
            if ext == "rs" {
                let path_str = entry.path().to_string_lossy().to_string();
                let code = fs::read_to_string(entry.path())?;
                let inst = InstructionPair {
                    instruction: format!("Explain or adapt this Rust snippet from {}", path_str),
                    input: String::new(),
                    output: code,
                };
                lines.push(serde_json::to_string(&inst)?);
            }
        }
    }
    fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap_or_else(|| std::path::Path::new(".")))?;
    fs::write(out_path, lines.join("\n"))?;
    Ok(())
}
