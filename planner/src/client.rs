use async_trait::async_trait;
use reqwest::{Client, Url};
use std::time::Duration;
use crate::types::{Task, PlannerError, ExecutionTrace};

/// Core trait defining the planner service interface
#[async_trait]
pub trait PlannerService: Send + Sync + 'static {
    async fn decompose_task(&self, objective: &str, context: &[String]) -> Result<Task, PlannerError>;
    async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), PlannerError>;
}

/// LaVague client configuration
#[derive(Clone, Debug)]
pub struct LaVagueConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub user_agent: String,
}

impl Default for LaVagueConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8000".to_string(),
            api_key: None,
            timeout: Duration::from_secs(30),
            user_agent: format!("useragent_id/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Real implementation of the LaVague client
#[derive(Clone)]
pub struct LaVagueClient {
    config: LaVagueConfig,
    client: Client,
}

impl LaVagueClient {
    /// Create a new LaVague client with the given configuration
    pub fn new(config: LaVagueConfig) -> Result<Self, PlannerError> {
        // Validate the endpoint URL
        let _url = Url::parse(&config.endpoint)
            .map_err(|e| PlannerError::InvalidResponse(format!("Invalid endpoint URL: {}", e)))?;
        
        // Begin client builder
        let mut client_builder = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent);
        
    // If TLS certificate path is provided, configure certificate validation (omitted for now)

        // Build the HTTP client
        let client = client_builder.build()
            .map_err(|e| PlannerError::Internal(format!("Failed to build HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }
    
    /// Internal helper to make authenticated requests
    async fn make_request<T: serde::Serialize>(
        &self, 
        path: &str, 
        payload: &T
    ) -> Result<reqwest::Response, PlannerError> {
        let url = format!("{}{}", self.config.endpoint, path);
        
        let mut req = self.client
            .post(&url)
            .json(payload);
        
        // Add authentication if configured
        if let Some(api_key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
        
        // Execute the request
        let resp = req.send().await
            .map_err(|e| match e.is_timeout() {
                true => PlannerError::Other(format!("Request timed out: {}", e)),
                false => PlannerError::Network(format!("Network error: {}", e)),
            })?;
        
        // Handle common error status codes
        match resp.status() {
            status if status.is_success() => Ok(resp),
            status if status.as_u16() == 401 || status.as_u16() == 403 => {
                Err(PlannerError::Authentication(format!("Authentication failed: {}", status)))
            },
            status if status.as_u16() == 429 => {
                Err(PlannerError::RateLimited)
            },
            status if status.as_u16() == 503 || status.as_u16() == 502 => {
                Err(PlannerError::ServiceUnavailable(format!("Service unavailable: {}", status)))
            },
            status => {
                // Try to get error details from body
                let error_text = resp.text().await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                
                Err(PlannerError::InvalidResponse(format!(
                    "Unexpected status code: {} - {}", 
                    status, 
                    error_text
                )))
            }
        }
    }
    
    /// Decompose a task using the LaVague API
    pub async fn decompose_task(&self, objective: &str, context_keys: &[String]) -> Result<Task, PlannerError> {
        let payload = serde_json::json!({
            "objective": objective,
            "context_keys": context_keys,
        });
        
        let resp = self.make_request("/api/v1/decompose", &payload).await?;
        
        // Parse the response
        let task: Task = resp.json().await
            .map_err(|e| PlannerError::InvalidResponse(format!("Failed to parse response: {}", e)))?;
        
        Ok(task)
    }
    
    /// Submit execution feedback to the LaVague API
    pub async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), PlannerError> {
        let resp = self.make_request("/api/v1/feedback", trace).await?;
        
        // For feedback, we just need to check that it was accepted
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(PlannerError::InvalidResponse(format!(
                "Failed to submit feedback: {}", 
                resp.status()
            )))
        }
    }
}

#[async_trait]
impl PlannerService for LaVagueClient {
    async fn decompose_task(&self, objective: &str, context: &[String]) -> Result<Task, PlannerError> {
        self.decompose_task(objective, context).await
    }
    
    async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), PlannerError> {
        self.submit_feedback(trace).await
    }
}

/// Feature-flagged mock implementation for development and testing
#[cfg(feature = "mock")]
pub mod mock {
    use super::*;
    use uuid::Uuid;
    use std::sync::{Arc, Mutex};
    use chrono::Utc;
    
    /// Mock implementation of the planner service
    pub struct MockPlannerService {
        tasks: Arc<Mutex<Vec<Task>>>,
        traces: Arc<Mutex<Vec<ExecutionTrace>>>,
        error_mode: bool,
    }
    
    impl MockPlannerService {
        /// Create a new mock planner service
        pub fn new() -> Self {
            Self {
                tasks: Arc::new(Mutex::new(Vec::new())),
                traces: Arc::new(Mutex::new(Vec::new())),
                error_mode: false,
            }
        }
        
        /// Set the error mode for testing error handling
        pub fn set_error_mode(&mut self, error_mode: bool) {
            self.error_mode = error_mode;
        }
        
        /// Get all recorded tasks
        pub fn get_tasks(&self) -> Vec<Task> {
            self.tasks.lock().unwrap().clone()
        }
        
        /// Get all recorded traces
        pub fn get_traces(&self) -> Vec<ExecutionTrace> {
            self.traces.lock().unwrap().clone()
        }
    }
    
    #[async_trait]
    impl PlannerService for MockPlannerService {
        async fn decompose_task(&self, objective: &str, _context: &[String]) -> Result<Task, PlannerError> {
            if self.error_mode {
                return Err(PlannerError::ServiceUnavailable("Mock service in error mode".to_string()));
            }
            
            // Create a mock task
            let task = Task {
                id: Uuid::new_v4().to_string(),
                objective: objective.to_string(),
                subtasks: vec![
                    Subtask {
                        id: Uuid::new_v4().to_string(),
                        objective: format!("Search for information about {}", objective),
                        required_agent: "Scrape".to_string(),
                        input_keys: vec!["query".to_string()],
                        output_keys: vec!["results".to_string()],
                        status: SubtaskStatus::Pending,
                        dependencies: vec![],
                    },
                    Subtask {
                        id: Uuid::new_v4().to_string(),
                        objective: format!("Process information about {}", objective),
                        required_agent: "Process".to_string(),
                        input_keys: vec!["results".to_string()],
                        output_keys: vec!["summary".to_string()],
                        status: SubtaskStatus::Pending,
                        dependencies: vec![],
                    },
                ],
                metadata: TaskMetadata {
                    created_at: Some(Utc::now().to_rfc3339()),
                    planner: Some("mock".to_string()),
                    cached: false,
                    version: Some("0.1.0".to_string()),
                },
            };
            
            // Record the task
            self.tasks.lock().unwrap().push(task.clone());
            
            Ok(task)
        }
        
        async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), PlannerError> {
            if self.error_mode {
                return Err(PlannerError::ServiceUnavailable("Mock service in error mode".to_string()));
            }
            
            // Record the trace
            self.traces.lock().unwrap().push(trace.clone());
            
            Ok(())
        }
    }
}

/// Convenience function to create a LaVague client from environment variables
pub fn client_from_env() -> Result<LaVagueClient, PlannerError> {
    let endpoint = std::env::var("LAVAGUE_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8000".to_string());
    
    let api_key = std::env::var("LAVAGUE_API_KEY").ok();
    
    let timeout = std::env::var("LAVAGUE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    
    let config = LaVagueConfig {
        endpoint,
        api_key,
        timeout: Duration::from_secs(timeout),
        user_agent: format!("useragent_id/{}", env!("CARGO_PKG_VERSION")),
    };
    
    LaVagueClient::new(config)
}
