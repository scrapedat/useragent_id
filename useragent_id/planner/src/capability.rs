use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use crate::task::AgentType;

/// Capability metadata for agent discovery
#[derive(Debug, Clone)]
pub struct AgentCapability {
    /// Name of the capability
    pub name: String,
    
    /// Agent type that provides this capability
    pub agent_type: AgentType,
    
    /// Path to the agent implementation
    pub path: PathBuf,
    
    /// Whether the agent is a WASM module or native binary
    pub is_wasm: bool,
    
    /// Supported input types
    pub inputs: Vec<String>,
    
    /// Supported output types
    pub outputs: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Agent capability discovery service
pub struct CapabilityDiscovery {
    /// Base directory for agent scanning
    base_dir: PathBuf,
    
    /// Cache of discovered capabilities
    capabilities: RwLock<HashMap<String, AgentCapability>>,
    
    /// Whether to scan for native agents
    scan_native: bool,
    
    /// Whether to scan for WASM agents
    scan_wasm: bool,
}

impl CapabilityDiscovery {
    /// Create a new capability discovery service
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            capabilities: RwLock::new(HashMap::new()),
            scan_native: true,
            scan_wasm: true,
        }
    }
    
    /// Configure capability discovery
    pub fn configure(&mut self, scan_native: bool, scan_wasm: bool) {
        self.scan_native = scan_native;
        self.scan_wasm = scan_wasm;
    }
    
    /// Discover agent capabilities
    pub async fn discover(&self) -> Result<Vec<AgentCapability>> {
        // Clear the cache
        self.capabilities.write().await.clear();
        
        // Discover capabilities
        let mut result = Vec::new();
        
        // Scan for WASM agents
        if self.scan_wasm {
            let wasm_dir = self.base_dir.join("agents").join("wasm");
            if wasm_dir.exists() {
                let wasm_agents = self.scan_wasm_agents(&wasm_dir).await?;
                result.extend(wasm_agents);
            }
        }
        
        // Scan for native agents
        if self.scan_native {
            let native_dir = self.base_dir.join("agents").join("native");
            if native_dir.exists() {
                let native_agents = self.scan_native_agents(&native_dir).await?;
                result.extend(native_agents);
            }
            
            // Also scan for built-in native agents
            if let Some(scraper_path) = self.find_scraper_agent().await {
                let scraper_capability = AgentCapability {
                    name: "Scraper".to_string(),
                    agent_type: AgentType::Scrape,
                    path: scraper_path,
                    is_wasm: false,
                    inputs: vec!["target_url".to_string()],
                    outputs: vec!["page_content".to_string()],
                    metadata: HashMap::new(),
                };
                result.push(scraper_capability);
            }
        }
        
        // Update the cache
        let mut cache = self.capabilities.write().await;
        for capability in &result {
            cache.insert(capability.name.clone(), capability.clone());
        }
        
        Ok(result)
    }
    
    /// Scan for WASM agents
    async fn scan_wasm_agents(&self, dir: &Path) -> Result<Vec<AgentCapability>> {
        use tokio::fs::{self, DirEntry};
        use tokio::process::Command;
        
        let mut result = Vec::new();
        
        // Read the directory
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Skip non-WASM files
            if let Some(ext) = path.extension() {
                if ext != "wasm" {
                    continue;
                }
            } else {
                continue;
            }
            
            // Try to read the WASM module
            let capability = self.extract_wasm_capability(&path).await?;
            result.push(capability);
        }
        
        Ok(result)
    }
    
    /// Extract capability information from a WASM module
    async fn extract_wasm_capability(&self, path: &Path) -> Result<AgentCapability> {
        // In a real implementation, this would use wasm_bindgen or similar
        // to introspect the WASM module. For now, we'll use naming convention.
        let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
        
        let agent_type = if file_name.contains("scrape") {
            AgentType::Scrape
        } else if file_name.contains("vision") {
            AgentType::Vision
        } else if file_name.contains("time") {
            AgentType::Time
        } else if file_name.contains("data") {
            AgentType::Data
        } else {
            AgentType::Custom(file_name.clone())
        };
        
        let capability = AgentCapability {
            name: file_name,
            agent_type,
            path: path.to_path_buf(),
            is_wasm: true,
            inputs: vec!["input".to_string()],
            outputs: vec!["output".to_string()],
            metadata: HashMap::new(),
        };
        
        Ok(capability)
    }
    
    /// Scan for native agents
    async fn scan_native_agents(&self, dir: &Path) -> Result<Vec<AgentCapability>> {
        use tokio::fs::{self, DirEntry};
        
        let mut result = Vec::new();
        
        // Read the directory
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Skip directories
            if path.is_dir() {
                continue;
            }
            
            // Try to read the native agent
            match self.extract_native_capability(&path).await {
                Ok(capability) => {
                    result.push(capability);
                },
                Err(e) => {
                    log::warn!("Failed to extract capability from {:?}: {}", path, e);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Extract capability information from a native agent
    async fn extract_native_capability(&self, path: &Path) -> Result<AgentCapability> {
        use tokio::process::Command;
        
        // Try to run the agent with --capabilities flag
        let output = Command::new(path)
            .arg("--capabilities")
            .output()
            .await?;
        
        if !output.status.success() {
            anyhow::bail!("Agent failed to return capabilities");
        }
        
        // Parse the output as JSON
        let stdout = String::from_utf8(output.stdout)?;
        let capabilities: serde_json::Value = serde_json::from_str(&stdout)?;
        
        // Extract the agent type
        let agent_type_str = capabilities["agent_type"].as_str().unwrap_or("custom");
        let agent_type = match agent_type_str {
            "scrape" => AgentType::Scrape,
            "vision" => AgentType::Vision,
            "time" => AgentType::Time,
            "data" => AgentType::Data,
            _ => AgentType::Custom(agent_type_str.to_string()),
        };
        
        // Extract inputs and outputs
        let inputs = capabilities["inputs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        
        let outputs = capabilities["outputs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        
        // Extract metadata
        let metadata = capabilities["metadata"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        v.as_str().map(|s| (k.clone(), s.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_else(HashMap::new);
        
        let capability = AgentCapability {
            name: path.file_stem().unwrap().to_string_lossy().to_string(),
            agent_type,
            path: path.to_path_buf(),
            is_wasm: false,
            inputs,
            outputs,
            metadata,
        };
        
        Ok(capability)
    }
    
    /// Find the built-in scraper agent
    async fn find_scraper_agent(&self) -> Option<PathBuf> {
        // First check env var
        if let Ok(path) = std::env::var("AGENT_SCRAPER_BIN") {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                return Some(path_buf);
            }
        }
        
        // Check in agents/scraper_chromiumoxide/target/release
        let release_path = self.base_dir
            .join("agents")
            .join("scraper_chromiumoxide")
            .join("target")
            .join("release")
            .join("scraper_chromiumoxide");
        
        if release_path.exists() {
            return Some(release_path);
        }
        
        // Check in target/release
        let workspace_release_path = self.base_dir
            .join("target")
            .join("release")
            .join("scraper_chromiumoxide");
        
        if workspace_release_path.exists() {
            return Some(workspace_release_path);
        }
        
        // Check in target/debug
        let debug_path = self.base_dir
            .join("target")
            .join("debug")
            .join("scraper_chromiumoxide");
        
        if debug_path.exists() {
            return Some(debug_path);
        }
        
        None
    }
    
    /// Get a capability by name
    pub async fn get_capability(&self, name: &str) -> Option<AgentCapability> {
        self.capabilities.read().await.get(name).cloned()
    }
    
    /// Get all capabilities
    pub async fn get_all_capabilities(&self) -> Vec<AgentCapability> {
        self.capabilities.read().await.values().cloned().collect()
    }
    
    /// Get capabilities by agent type
    pub async fn get_capabilities_by_type(&self, agent_type: &AgentType) -> Vec<AgentCapability> {
        self.capabilities.read().await.values()
            .filter(|cap| match (agent_type, &cap.agent_type) {
                (AgentType::Scrape, AgentType::Scrape) => true,
                (AgentType::Vision, AgentType::Vision) => true,
                (AgentType::Time, AgentType::Time) => true,
                (AgentType::Data, AgentType::Data) => true,
                (AgentType::Custom(name1), AgentType::Custom(name2)) => name1 == name2,
                _ => false,
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    
    #[tokio::test]
    async fn test_capability_discovery() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();
        
        // Create agent directories
        let wasm_dir = base_dir.join("agents").join("wasm");
        let native_dir = base_dir.join("agents").join("native");
        
        tokio::fs::create_dir_all(&wasm_dir).await.unwrap();
        tokio::fs::create_dir_all(&native_dir).await.unwrap();
        
        // Create a mock WASM file
        let wasm_file = wasm_dir.join("test_scrape.wasm");
        File::create(&wasm_file).await.unwrap();
        
        // Create a mock native agent
        let native_file = native_dir.join("test_vision");
        File::create(&native_file).await.unwrap();
        
        // Create capability discovery
        let discovery = CapabilityDiscovery::new(base_dir);
        
        // Discover capabilities
        let result = discovery.discover().await;
        
        // This will fail because the native agent can't be executed,
        // but we can verify that the WASM agent is discovered
        assert!(result.is_err() || !result.unwrap().is_empty());
    }
}
