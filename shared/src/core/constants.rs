use crate::core::config::ResourceLimits;

// Default resource limits for each app
pub const USER_MONITOR_LIMITS: ResourceLimits = ResourceLimits {
    max_memory_mb: 512,
    max_disk_space_mb: 2048,
};

pub const TASK_LEARNER_LIMITS: ResourceLimits = ResourceLimits {
    max_memory_mb: 1024,
    max_disk_space_mb: 1024,
};

pub const AGENT_RUNNER_LIMITS: ResourceLimits = ResourceLimits {
    max_memory_mb: 150,
    max_disk_space_mb: 250,
};

pub const AGENT_TRAINER_LIMITS: ResourceLimits = ResourceLimits {
    max_memory_mb: 4096,
    max_disk_space_mb: 5120,
};

// Data exchange formats
pub const FORMAT_VERSION: &str = "1.0.0";

// Supported data types for exchange
pub const DATA_TYPE_USER_ACTIONS: &str = "user_actions";
pub const DATA_TYPE_PATTERNS: &str = "behavior_patterns";
pub const DATA_TYPE_MODELS: &str = "agent_models";
pub const DATA_TYPE_TRAINING: &str = "training_data";
