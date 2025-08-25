use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Configuration for resource usage limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in megabytes
    pub max_memory_mb: usize,

    /// Maximum disk space usage in megabytes
    pub max_disk_space_mb: usize,
}

/// Status of an app's resource usage
pub struct ResourceStatus {
    pub memory_usage_mb: usize,
    pub disk_usage_mb: usize,
}

/// Shared configuration for all apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_id: String,
    pub data_dir: PathBuf,
    pub event_log_dir: PathBuf,
    pub learned_tasks_dir: PathBuf,
    /// Persistent Chromium/Chrome user-data-dir for web tasks
    pub browser_profile_dir: PathBuf,
    /// Chrome DevTools Protocol port for automation/recording
    pub cdp_port: u16,
    pub resource_limits: ResourceLimits,
    pub debug_mode: bool,
}

impl AppConfig {
    /// Creates a new AppConfig and ensures that the necessary directories exist.
    pub fn new(app_id: &str, data_dir_base: &Path, resource_limits: ResourceLimits, debug_mode: bool) -> Result<Self> {
        let data_dir = data_dir_base.join(app_id);
        let event_log_dir = data_dir.join("events");
        let learned_tasks_dir = data_dir.join("learned_tasks");
        let browser_profile_dir = data_dir.join("browser_profile");
        let cdp_port: u16 = 9222;

        // Create directories
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory for {}", app_id))?;
        std::fs::create_dir_all(&event_log_dir)
            .context("Failed to create event log directory")?;
        std::fs::create_dir_all(&learned_tasks_dir)
            .context("Failed to create learned tasks directory")?;
        std::fs::create_dir_all(&browser_profile_dir)
            .context("Failed to create browser profile directory")?;

        Ok(Self {
            app_id: app_id.to_string(),
            data_dir,
            event_log_dir,
            learned_tasks_dir,
            browser_profile_dir,
            cdp_port,
            resource_limits,
            debug_mode,
        })
    }

    /// Loads a default configuration for an application.
    pub fn load() -> Result<Self> {
        // For now, we'll create a default config. This could later load from a file.
        let data_dir = PathBuf::from("data");
        let limits = ResourceLimits {
            max_memory_mb: 1024,
            max_disk_space_mb: 2048,
        };
        // The app_id will be overwritten by each app, this is just a default.
        AppConfig::new("default", &data_dir, limits, true)
    }
}

/// Interface for apps to implement for standardized resource management
pub trait ResourceManaged {
    /// Cleanup resources used by the app
    fn cleanup_resources(&mut self) -> anyhow::Result<()>;
}

/// Data exchange format between apps
pub struct DataPacket {
    pub source_app: String,
    pub destination_app: String,
    pub data: Vec<u8>,
    pub size_bytes: usize,
}
