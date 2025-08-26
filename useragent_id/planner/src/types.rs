use serde::{Serialize, Deserialize};

/// Core Task structure that represents a decomposed objective
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub objective: String,
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub metadata: TaskMetadata,
}

/// Subtask representing a single unit of work for an agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subtask {
    pub id: String,
    pub objective: String,
    pub required_agent: String,
    pub input_keys: Vec<String>,
    pub output_keys: Vec<String>,
    #[serde(default)]
    pub status: SubtaskStatus,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Status of a subtask execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubtaskStatus {
    /// Subtask is waiting to be executed
    Pending,
    /// Subtask is currently being executed
    InProgress,
    /// Subtask has completed successfully
    Completed,
    /// Subtask execution failed
    Failed,
    /// Subtask was cancelled
    Cancelled,
}

impl Default for SubtaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Metadata for a task
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TaskMetadata {
    pub created_at: Option<String>,
    pub planner: Option<String>,
    pub cached: bool,
    pub version: Option<String>,
}

/// Execution trace for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub task_id: String,
    pub subtask_id: String,
    pub agent_type: String,
    pub status: SubtaskStatus,
    pub timestamp: String,
    pub outputs: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Error type for planner-related errors
#[derive(Debug, PartialEq)]
pub enum PlannerError {
    /// API request failed
    ApiError(String),
    /// Rate limiting error
    RateLimited,
    /// Authentication error
    AuthError,
    /// Circuit breaker is open
    CircuitOpen,
    /// Planning timed out
    Timeout,
    /// Security-related error
    SecurityError(String),
    /// Actor system error
    ActorError(String),
    /// Other error
    Other(String),
    /// Network error
    Network(String),
    /// Authentication error
    Authentication(String),
    /// Service unavailable
    ServiceUnavailable(String),
    /// Invalid response
    InvalidResponse(String),
    /// Internal error
    Internal(String),
    /// Actor unavailable
    ActorUnavailable(String),
    /// Response channel closed
    ResponseChannelClosed(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(s) => write!(f, "API error: {}", s),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::AuthError => write!(f, "Authentication error"),
            Self::CircuitOpen => write!(f, "Circuit breaker is open"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::SecurityError(s) => write!(f, "Security error: {}", s),
            Self::ActorError(s) => write!(f, "Actor error: {}", s),
            Self::Other(s) => write!(f, "Other error: {}", s),
            Self::Network(s) => write!(f, "Network error: {}", s),
            Self::Authentication(s) => write!(f, "Authentication error: {}", s),
            Self::ServiceUnavailable(s) => write!(f, "Service unavailable: {}", s),
            Self::InvalidResponse(s) => write!(f, "Invalid response: {}", s),
            Self::Internal(s) => write!(f, "Internal error: {}", s),
            Self::ActorUnavailable(s) => write!(f, "Actor unavailable: {}", s),
            Self::ResponseChannelClosed(s) => write!(f, "Response channel closed: {}", s),
        }
    }
}

impl std::error::Error for PlannerError {}
