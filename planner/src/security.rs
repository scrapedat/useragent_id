use std::path::PathBuf;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Security configuration for LaVague client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// API key for authentication
    pub api_key: Option<String>,
    
    /// Path to TLS certificate for verification (optional)
    pub tls_cert_path: Option<PathBuf>,
    
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimit,
    
    /// Path to audit log (optional)
    pub audit_log_path: Option<PathBuf>,
    
    /// Whether to validate request/response data
    #[serde(default = "default_true")]
    pub validate_data: bool,
}

/// Default function for validate_data
fn default_true() -> bool {
    true
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            tls_cert_path: None,
            rate_limit: RateLimit::default(),
            audit_log_path: None,
            validate_data: true,
        }
    }
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum number of requests per minute
    #[serde(default = "default_rate_limit")]
    pub max_requests_per_minute: u32,
    
    /// Backoff strategy for rate limiting
    #[serde(default)]
    pub backoff_strategy: BackoffStrategy,
}

/// Default rate limit
fn default_rate_limit() -> u32 {
    60
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            max_requests_per_minute: default_rate_limit(),
            backoff_strategy: BackoffStrategy::default(),
        }
    }
}

/// Backoff strategies for rate limiting
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed {
        delay: u64,
    },
    
    /// Exponential backoff
    Exponential {
        initial: u64,
        multiplier: f64,
        max: u64,
    },
    
    /// Fibonacci backoff
    Fibonacci {
        initial: u64,
        max: u64,
    },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential {
            initial: 1000,
            multiplier: 2.0,
            max: 60000,
        }
    }
}

impl BackoffStrategy {
    /// Calculate backoff duration for a given attempt
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let millis = match self {
            Self::Fixed { delay } => *delay,
            Self::Exponential { initial, multiplier, max } => {
                let calculated = (*initial as f64 * multiplier.powf(attempt as f64)) as u64;
                calculated.min(*max)
            },
            Self::Fibonacci { initial, max } => {
                if attempt <= 1 {
                    *initial
                } else {
                    let mut a = *initial;
                    let mut b = *initial;
                    
                    for _ in 0..attempt - 1 {
                        let next = a + b;
                        a = b;
                        b = next;
                    }
                    
                    b.min(*max)
                }
            },
        };
        
        Duration::from_millis(millis)
    }
}

/// Rate limiter implementation
pub struct RateLimiter {
    limit: RateLimit,
    window: Duration,
    requests: Vec<std::time::Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(limit: RateLimit) -> Self {
        let max_requests = limit.max_requests_per_minute;
        Self {
            limit,
            window: Duration::from_secs(60),
            requests: Vec::with_capacity(max_requests as usize),
        }
    }
    
    /// Check if a request can be made, and if not, return the time to wait
    pub fn check(&mut self) -> Option<Duration> {
        let now = std::time::Instant::now();
        
        // Remove requests outside the window
        self.requests.retain(|t| now.duration_since(*t) < self.window);
        
        // Check if we're under the limit
        if self.requests.len() < self.limit.max_requests_per_minute as usize {
            self.requests.push(now);
            None
        } else {
            // Calculate time to wait
            let oldest = self.requests[0];
            let time_passed = now.duration_since(oldest);
            
            if time_passed < self.window {
                Some(self.window - time_passed)
            } else {
                self.requests.remove(0);
                self.requests.push(now);
                None
            }
        }
    }
    
    /// Wait until a request can be made
    pub async fn wait(&mut self) {
        if let Some(wait_time) = self.check() {
            tokio::time::sleep(wait_time).await;
        }
    }
}

/// Audit logger for security events
pub struct AuditLogger {
    path: PathBuf,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self { path })
    }
    
    /// Log an audit event
    pub fn log(&self, event_type: &str, details: &serde_json::Value) -> Result<(), std::io::Error> {
        let now = chrono::Utc::now();
        
        let event = serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "type": event_type,
            "details": details,
        });
        
        // Append to the audit log
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        
        writeln!(file, "{}", serde_json::to_string(&event)?)?;
        
        Ok(())
    }
}

/// Data sanitizer for input validation
pub struct DataSanitizer;

impl DataSanitizer {
    /// Sanitize a string for use in LaVague API
    pub fn sanitize_string(input: &str) -> String {
        // Remove control characters
        input.chars()
            .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
            .collect()
    }
    
    /// Validate a task objective
    pub fn validate_objective(objective: &str) -> Result<(), &'static str> {
        if objective.is_empty() {
            return Err("Objective cannot be empty");
        }
        
        if objective.len() > 1000 {
            return Err("Objective is too long (max 1000 characters)");
        }
        
        Ok(())
    }
    
    /// Validate context keys
    pub fn validate_context_keys(keys: &[String]) -> Result<(), &'static str> {
        if keys.len() > 100 {
            return Err("Too many context keys (max 100)");
        }
        
        for key in keys {
            if key.is_empty() {
                return Err("Context key cannot be empty");
            }
            
            if key.len() > 100 {
                return Err("Context key is too long (max 100 characters)");
            }
        }
        
        Ok(())
    }
}
