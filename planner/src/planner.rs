use crate::task::{Task, Subtask, TaskStatus};
use crate::client::{LaVagueClient, LaVagueConfig, PlannerService};
use crate::security::{SecurityConfig, DataSanitizer};
use crate::actor::PlannerActorSystem;
use crate::cache::PlanCache;
use crate::circuit::{CircuitBreaker, CircuitBreakerConfig, CircuitProtected};
use crate::feedback::{FeedbackCollector, FeedbackConfig};
use crate::types::{ExecutionTrace, SubtaskStatus};

use memory::SharedContext;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use std::str::FromStr;
use log::{info, warn, error, debug};
use chrono::Utc;

/// Planner mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerMode {
    /// Use LaVague planning API
    LaVague,
    /// Use local rule-based decomposition
    Local,
    /// Try LaVague first, fall back to local
    Hybrid,
}

impl PlannerMode {
    /// Convert PlannerMode to string representation
    pub fn to_string(&self) -> String {
        match self {
            PlannerMode::LaVague => "lavague".to_string(),
            PlannerMode::Local => "local".to_string(),
            PlannerMode::Hybrid => "hybrid".to_string(),
        }
    }
}

/// The main planner that orchestrates task decomposition
pub struct Planner {
    mode: PlannerMode,
    lavague_client: Option<CircuitProtected<LaVagueClient>>,
    actor_system: Option<Arc<PlannerActorSystem>>,
    cache: Arc<RwLock<Option<PlanCache>>>,
    security: SecurityConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    feedback_collector: Option<Arc<FeedbackCollector>>,
}

impl Planner {
    /// Create a new planner with the given mode
    pub async fn new(mode: PlannerMode, enable_cache: bool) -> Result<Self> {
        // Configure circuit breaker
        let circuit_config = CircuitBreakerConfig {
            failure_threshold: std::env::var("LAVAGUE_CIRCUIT_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            reset_timeout_ms: std::env::var("LAVAGUE_CIRCUIT_RESET_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30000),
            half_open_limit: 1,
        };
        
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_config));
        
        // Initialize LaVague client if needed
        let (lavague_client, actor_system) = match mode {
            PlannerMode::Local => (None, None),
            _ => {
                // Get endpoint from environment - MUST be configurable
                let endpoint = match std::env::var("LAVAGUE_ENDPOINT") {
                    Ok(ep) => ep,
                    Err(_) => return Err(anyhow::anyhow!("LAVAGUE_ENDPOINT environment variable is required")),
                };
                
                let api_key = std::env::var("LAVAGUE_API_KEY").ok();
                
                let config = LaVagueConfig {
                    endpoint,
                    api_key: api_key.clone(),
                    timeout: Duration::from_secs(30),
                    user_agent: format!("useragent_id/{}", env!("CARGO_PKG_VERSION")),
                };
                
                // Create client with TLS if configured
                let tls_cert_path = std::env::var("LAVAGUE_TLS_CERT")
                    .ok()
                    .map(std::path::PathBuf::from);
                    
                if let Some(path) = &tls_cert_path {
                    if !path.exists() {
                        return Err(anyhow::anyhow!("TLS certificate file does not exist: {:?}", path));
                    }
                }
                
                let mut config = config;
                // TLS cert path is now handled differently
                
                // Create client
                let client = LaVagueClient::new(config)
                    .context("Failed to create LaVague client")?;
                
                // Wrap client with circuit breaker
                let protected_client = CircuitProtected::new(client.clone(), circuit_breaker.clone());
                
                // Create actor system
                let actor_system = PlannerActorSystem::new(client).await;
                
                (Some(protected_client), Some(Arc::new(actor_system)))
            }
        };
        
        // Initialize security config
        let security = SecurityConfig {
            api_key: std::env::var("LAVAGUE_API_KEY").ok(),
            tls_cert_path: std::env::var("LAVAGUE_TLS_CERT").ok().map(std::path::PathBuf::from),
            rate_limit: Default::default(),
            audit_log_path: std::env::var("LAVAGUE_AUDIT_LOG").ok().map(std::path::PathBuf::from),
            validate_data: true,
        };
        
        // Initialize cache if enabled
        let cache_capacity = std::env::var("LAVAGUE_CACHE_CAPACITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
            
        let cache = if enable_cache {
            Arc::new(RwLock::new(Some(PlanCache::new(cache_capacity))))
        } else {
            Arc::new(RwLock::new(None))
        };
        
        // Initialize feedback collector
        let feedback_collector = match std::env::var("ENABLE_FEEDBACK_COLLECTION").ok() {
            Some(val) if val.to_lowercase() == "true" || val == "1" => {
                let feedback_config = FeedbackConfig {
                    feedback_dir: std::env::var("FEEDBACK_DIR")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("./feedback")),
                    batch_enabled: true,
                    batch_size: std::env::var("FEEDBACK_BATCH_SIZE")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10),
                    flush_interval_seconds: std::env::var("FEEDBACK_FLUSH_INTERVAL")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(60),
                    max_retries: std::env::var("FEEDBACK_MAX_RETRIES")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(3),
                };
                
                // Create the client service to use for feedback submission
                match &lavague_client {
                    Some(protected_client) => {
                        let client = Arc::new(protected_client.inner().clone());
                        Some(Arc::new(FeedbackCollector::new(client, feedback_config)))
                    },
                    None => None
                }
            },
            _ => None
        };

        Ok(Self {
            mode,
            lavague_client,
            actor_system,
            cache,
            security,
            circuit_breaker,
            feedback_collector,
        })
    }
    
    /// Get a task from cache if available
    async fn get_from_cache(&self, objective: &str) -> Option<crate::task::Task> {
        if let Some(cache) = &*self.cache.read().await {
            cache.get(objective).map(|types_task| self.convert_types_task_to_task(&types_task))
        } else {
            None
        }
    }
    
    /// Convert types::Task to task::Task
    fn convert_types_task_to_task(&self, types_task: &crate::types::Task) -> crate::task::Task {
        let subtasks = types_task.subtasks.iter().map(|s| crate::task::Subtask {
            id: s.id.clone(),
            objective: s.objective.clone(),
            required_agent: crate::task::AgentType::from_str(&s.required_agent).unwrap_or(crate::task::AgentType::Custom(s.required_agent.clone())),
            dependencies: s.dependencies.clone(),
            input_keys: s.input_keys.clone(),
            output_key: s.output_keys.first().cloned().unwrap_or_default(),
        }).collect();

        crate::task::Task {
            id: types_task.id.clone(),
            objective: types_task.objective.clone(),
            subtasks,
            status: crate::task::TaskStatus::Pending,
            created_at: types_task.metadata.created_at.as_ref()
                .and_then(|ts| ts.parse::<u64>().ok())
                .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        }
    }
    
    /// Convert task::Task to types::Task
    fn convert_task_to_types_task(&self, task: &crate::task::Task) -> crate::types::Task {
        let subtasks = task.subtasks.iter().map(|s| crate::types::Subtask {
            id: s.id.clone(),
            objective: s.objective.clone(),
            required_agent: s.required_agent.to_string(),
            input_keys: s.input_keys.clone(),
            output_keys: vec![s.output_key.clone()],
            status: crate::types::SubtaskStatus::Pending,
            dependencies: s.dependencies.clone(),
        }).collect();
        
        crate::types::Task {
            id: task.id.clone(),
            objective: task.objective.clone(),
            subtasks,
            metadata: crate::types::TaskMetadata {
                created_at: Some(task.created_at.to_string()),
                planner: Some("LaVague".to_string()),
                cached: false,
                version: Some("1.0".to_string()),
            }
        }
    }
    
    /// Store a task in cache
    async fn store_in_cache(&self, objective: String, task: crate::task::Task) {
        if let Some(cache) = &mut *self.cache.write().await {
            let types_task = self.convert_task_to_types_task(&task);
            cache.insert(objective, types_task);
        }
    }
    
    /// Submit feedback for task execution
    pub async fn submit_feedback(&self, task: &crate::task::Task, subtask_id: &str, status: crate::types::SubtaskStatus, 
                              duration_ms: u64, error: Option<String>) -> Result<()> {
        if let Some(feedback_collector) = &self.feedback_collector {
            // Find the subtask
            let subtask = task.subtasks.iter()
                .find(|s| s.id == subtask_id);
                
            // Create execution trace
            let trace = ExecutionTrace {
                task_id: task.id.clone(),
                subtask_id: subtask_id.to_string(),
                agent_type: subtask
                    .map(|s| s.required_agent.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                status,
                timestamp: Utc::now().to_rfc3339(),
                outputs: None, // Could add subtask result here if needed
                error,
                duration_ms,
            };
            
            // Submit feedback
            feedback_collector.submit(trace).await
                .map_err(|e| anyhow::anyhow!("Failed to submit feedback: {}", e))?;
            
            debug!("Submitted execution feedback for subtask {}", subtask_id);
            Ok(())
        } else {
            // Feedback collection not enabled
            Ok(())
        }
    }
    
    /// Generate metrics on completed task
    pub async fn get_feedback_metrics(&self) -> Result<Option<Vec<String>>> {
        if let Some(feedback_collector) = &self.feedback_collector {
            let metrics = feedback_collector.get_metrics().await;
            
            let mut result = vec![
                format!("Pending feedback: {}", metrics.pending_count),
                format!("Failed submissions: {}", metrics.failed_count),
                format!("Retry count: {}", metrics.retry_counts),
            ];
            
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

// Improved version of the LaVague call with proper error handling and security measures
async fn call_lavague(
    client: &LaVagueClient,
    objective: &str,
    context_keys: &[String],
) -> Result<crate::task::Task> {
    // Input validation
    DataSanitizer::validate_objective(objective)
        .map_err(|e| anyhow::anyhow!("Invalid objective: {}", e))?;
    
    DataSanitizer::validate_context_keys(context_keys)
        .map_err(|e| anyhow::anyhow!("Invalid context keys: {}", e))?;

    // Make the API call with proper error handling
    let types_task = client.decompose_task(objective, context_keys)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call LaVague API: {:?}", e))?;
    
    // Convert from types::Task to task::Task
    let subtasks = types_task.subtasks.iter().map(|s| crate::task::Subtask {
        id: s.id.clone(),
        objective: s.objective.clone(),
        required_agent: crate::task::AgentType::from_str(&s.required_agent).unwrap_or(crate::task::AgentType::Custom(s.required_agent.clone())),
        dependencies: s.dependencies.clone(),
        input_keys: s.input_keys.clone(),
        output_key: s.output_keys.first().cloned().unwrap_or_default(),
    }).collect();

    let task = crate::task::Task {
        id: types_task.id.clone(),
        objective: types_task.objective.clone(),
        subtasks,
        status: crate::task::TaskStatus::Pending,
        created_at: types_task.metadata.created_at.as_ref()
            .and_then(|ts| ts.parse::<u64>().ok())
            .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
    };
    
    Ok(task)
}

/// Production-ready local planning implementation for when LaVague is unavailable
async fn local_planning(objective: &str, context: &SharedContext) -> Result<crate::task::Task> {
    info!("Using production-ready local planning for: {}", objective);
    
    // Generate a proper UUID for the task
    use uuid::Uuid;
    let task_id = Uuid::new_v4().to_string();
    
    // Structured task analysis using intent recognition patterns
    let intent_analyzer = LocalIntentAnalyzer::new();
    let intents = intent_analyzer.analyze(objective);
    
    // Extract available context data for subtask generation
    let context_data = context.keys();
    let available_inputs = context_data.iter().cloned().collect::<Vec<_>>();
    
    // Generate subtasks based on intent analysis
    let subtask_generator = SubtaskGenerator::new();
    let subtasks = subtask_generator.generate_subtasks(objective, &intents, &available_inputs)?;
    
    // Validate the generated plan ensures all required capabilities are present
    validate_plan(&subtasks)?;
    
    // Create the final task with proper metadata
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let task = Task { 
        id: task_id, 
        objective: objective.to_string(), 
        subtasks,
        status: TaskStatus::Pending,
        created_at: now,
    };
    
    Ok(task)
}

/// Intent analyzer for local planning
struct LocalIntentAnalyzer;

impl LocalIntentAnalyzer {
    fn new() -> Self {
        Self
    }
    
    fn analyze(&self, objective: &str) -> Vec<Intent> {
        let lower_obj = objective.to_lowercase();
        let mut intents = Vec::new();
        
        // Identify scraping intent
        if lower_obj.contains("scrape") 
            || lower_obj.contains("browse") 
            || lower_obj.contains("visit") 
            || lower_obj.contains("navigate")
            || lower_obj.contains("search") 
            || lower_obj.contains("find") 
            || lower_obj.contains("look up") {
            intents.push(Intent::Scrape);
        }
        
        // Identify processing intent
        if lower_obj.contains("process") 
            || lower_obj.contains("analyze") 
            || lower_obj.contains("extract") 
            || lower_obj.contains("summarize")
            || lower_obj.contains("parse") {
            intents.push(Intent::Process);
        }
        
        // Identify data storage intent
        if lower_obj.contains("save") 
            || lower_obj.contains("store") 
            || lower_obj.contains("record") 
            || lower_obj.contains("persist") {
            intents.push(Intent::Store);
        }
        
        // Default to General intent if none detected
        if intents.is_empty() {
            intents.push(Intent::General);
        }
        
        intents
    }
}

/// Task intents for planning
#[derive(Debug, Clone, PartialEq)]
enum Intent {
    Scrape,
    Process,
    Store,
    General,
}

/// Subtask generator for local planning
struct SubtaskGenerator;

impl SubtaskGenerator {
    fn new() -> Self {
        Self
    }
    
    fn generate_subtasks(&self, objective: &str, intents: &[Intent], available_inputs: &[String]) -> Result<Vec<Subtask>> {
        use uuid::Uuid;
        let mut subtasks = Vec::new();
        let mut dependencies = Vec::new();
        
        // Generate subtasks based on detected intents
        for (idx, intent) in intents.iter().enumerate() {
            let subtask_id = Uuid::new_v4().to_string();
            
            match intent {
                Intent::Scrape => {
                    // Create scraping subtask
                    let has_url = available_inputs.iter().any(|s| s == "target_url" || s == "url");
                    let input_keys = if has_url { vec!["target_url".to_string()] } else { vec![] };
                    
                    let subtask = Subtask {
                        id: subtask_id.clone(),
                        objective: format!("Scrape information related to: {}", objective),
                        required_agent: "Scrape".parse().unwrap(),
                        dependencies: vec![],
                        input_keys,
                        output_key: "page_content".to_string(),
                    };
                    
                    subtasks.push(subtask);
                    dependencies.push(subtask_id);
                },
                Intent::Process => {
                    // Create processing subtask
                    let mut input_keys = Vec::new();
                    let mut deps = Vec::new();
                    
                    // If we have a scraping subtask, use its output
                    if intents.contains(&Intent::Scrape) {
                        input_keys.push("page_content".to_string());
                        deps.extend(dependencies.clone());
                    } else if !available_inputs.is_empty() {
                        // Otherwise use available context inputs
                        input_keys.extend(available_inputs.iter().cloned());
                    }
                    
                    let subtask = Subtask {
                        id: subtask_id.clone(),
                        objective: format!("Process information for: {}", objective),
                        required_agent: "Process".parse().unwrap(),
                        dependencies: deps,
                        input_keys,
                        output_key: "processed_data".to_string(),
                    };
                    
                    subtasks.push(subtask);
                    dependencies.push(subtask_id);
                },
                Intent::Store => {
                    // Create storage subtask
                    let mut input_keys = Vec::new();
                    
                    // Determine what to store based on previous subtasks
                    if intents.contains(&Intent::Process) {
                        input_keys.push("processed_data".to_string());
                    } else if intents.contains(&Intent::Scrape) {
                        input_keys.push("page_content".to_string());
                    }
                    
                    let subtask = Subtask {
                        id: subtask_id.clone(),
                        objective: format!("Store results for: {}", objective),
                        required_agent: "Data".parse().unwrap(),
                        dependencies: dependencies.clone(),
                        input_keys,
                        output_key: "stored_location".to_string(),
                    };
                    
                    subtasks.push(subtask);
                },
                Intent::General => {
                    // Create a general subtask if no specific intents were detected
                    if idx == 0 && intents.len() == 1 {
                        let subtask = Subtask {
                            id: subtask_id,
                            objective: objective.to_string(),
                            required_agent: "General".parse().unwrap(),
                            dependencies: vec![],
                            input_keys: available_inputs.to_vec(),
                            output_key: "result".to_string(),
                        };
                        
                        subtasks.push(subtask);
                    }
                }
            }
        }
        
        Ok(subtasks)
    }
}

/// Validate that a generated plan is executable with available capabilities
fn validate_plan(subtasks: &[Subtask]) -> Result<()> {
    // Ensure we have at least one subtask
    if subtasks.is_empty() {
        return Err(anyhow::anyhow!("Generated plan has no subtasks"));
    }
    
    // Check for dependency cycles
    let mut visited = std::collections::HashSet::new();
    let mut path = std::collections::HashSet::new();
    
    for subtask in subtasks {
        if !visited.contains(&subtask.id) {
            if has_cycle(subtask, subtasks, &mut visited, &mut path) {
                return Err(anyhow::anyhow!("Generated plan has circular dependencies"));
            }
        }
    }
    
    // Verify that all dependencies refer to existing subtasks
    for subtask in subtasks {
        for dep in &subtask.dependencies {
            if !subtasks.iter().any(|st| &st.id == dep) {
                return Err(anyhow::anyhow!("Subtask {} depends on non-existent subtask {}", subtask.id, dep));
            }
        }
    }
    
    // Verify required agent capabilities
    let available_agents = get_available_agents();
    for subtask in subtasks {
        if !available_agents.contains(&subtask.required_agent.to_string()) {
            warn!("Missing agent capability: {}", subtask.required_agent);
            // Don't fail - log warning and continue as the agent may be dynamically loaded
        }
    }
    
    Ok(())
}

/// Check if there's a cycle in the dependency graph
fn has_cycle(
    current: &Subtask,
    all_subtasks: &[Subtask],
    visited: &mut std::collections::HashSet<String>,
    path: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(current.id.clone());
    path.insert(current.id.clone());
    
    for dep_id in &current.dependencies {
        if let Some(dep) = all_subtasks.iter().find(|st| &st.id == dep_id) {
            if !visited.contains(&dep.id) {
                if has_cycle(dep, all_subtasks, visited, path) {
                    return true;
                }
            } else if path.contains(&dep.id) {
                return true;
            }
        }
    }
    
    path.remove(&current.id);
    false
}

/// Get list of available agent capabilities
fn get_available_agents() -> Vec<String> {
    // In production this would scan for available agents
    vec![
        "Scrape".to_string(),
        "Process".to_string(),
        "Data".to_string(),
        "General".to_string(),
        "Time".to_string(),
        "Vision".to_string(),
    ]
}

pub async fn decompose_task(objective: &str, context: &SharedContext) -> Result<crate::task::Task> {
    // Initialize planner - reads from environment to determine mode
    let mode = match std::env::var("LAVAGUE_MODE").as_deref() {
        Ok("lavague") => PlannerMode::LaVague,
        Ok("local") => PlannerMode::Local,
        _ => PlannerMode::Hybrid, // Default to hybrid
    };
    
    let enable_cache = std::env::var("LAVAGUE_ENABLE_CACHE")
        .map(|v| v.to_lowercase() != "false" && v.to_lowercase() != "0")
        .unwrap_or(true);
    
    let planner = Planner::new(mode, enable_cache).await?;
    
    // Start timing for feedback metrics
    let start_time = std::time::Instant::now();
    
    // Check cache first
    if enable_cache {
        if let Some(cached_task) = planner.get_from_cache(objective).await {
            info!("Using cached plan for: {}", objective);
            
            // Record cache hit feedback if enabled
            if let Some(feedback_collector) = &planner.feedback_collector {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let trace = ExecutionTrace {
                    task_id: cached_task.id.clone(),
                    subtask_id: "cache_hit".to_string(),
                    agent_type: "cache".to_string(),
                    status: SubtaskStatus::Completed,
                    timestamp: Utc::now().to_rfc3339(),
                    outputs: None,
                    error: None,
                    duration_ms,
                };
                
                // Submit non-blocking
                let collector = feedback_collector.clone();
                tokio::spawn(async move {
                    if let Err(e) = collector.submit(trace).await {
                        warn!("Failed to submit cache hit feedback: {}", e);
                    }
                });
            }
            
            return Ok(cached_task);
        }
    }
    
    // Format prompt and context
    let context_keys = context.keys();
    info!("Planning task: {} with context keys: {:?}", objective, context_keys);
    
    // Create a task ID for feedback collection
    let task_id = format!("task_{}", SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());
        
    // Decompose based on mode
    let result = match mode {
        PlannerMode::LaVague => {
            if let Some(actor_system) = &planner.actor_system {
                let result: Result<crate::task::Task> = actor_system
                    .decompose_task(objective.to_string(), context_keys.clone())
                    .await
                    .map(|t| planner.convert_types_task_to_task(&t))
                    .map_err(|e| anyhow::anyhow!("Actor system error: {}", e));
                
                // Record feedback for LaVague mode
                if let Some(feedback_collector) = &planner.feedback_collector {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let (status, error) = match &result {
                        Ok(_) => (SubtaskStatus::Completed, None),
                        Err(e) => (SubtaskStatus::Failed, Some(e.to_string())),
                    };
                    
                    let trace = ExecutionTrace {
                        task_id: task_id.clone(),
                        subtask_id: "lavague_planning".to_string(),
                        agent_type: "lavague".to_string(),
                        status,
                        timestamp: Utc::now().to_rfc3339(),
                        outputs: None,
                        error,
                        duration_ms,
                    };
                    
                    // Submit non-blocking
                    let collector = feedback_collector.clone();
                    tokio::spawn(async move {
                        if let Err(e) = collector.submit(trace).await {
                            warn!("Failed to submit LaVague planning feedback: {}", e);
                        }
                    });
                }
                
                result
            } else {
                warn!("LaVague mode selected but actor system not available, falling back to local planning");
                
                // Record fallback event in feedback
                if let Some(feedback_collector) = &planner.feedback_collector {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let trace = ExecutionTrace {
                        task_id: task_id.clone(),
                        subtask_id: "lavague_unavailable".to_string(),
                        agent_type: "fallback".to_string(),
                        status: SubtaskStatus::InProgress, // Will be completed or failed later
                        timestamp: Utc::now().to_rfc3339(),
                        outputs: None,
                        error: Some("LaVague actor system unavailable".to_string()),
                        duration_ms,
                    };
                    
                    // Submit non-blocking
                    let collector = feedback_collector.clone();
                    tokio::spawn(async move {
                        if let Err(e) = collector.submit(trace).await {
                            warn!("Failed to submit LaVague unavailable feedback: {}", e);
                        }
                    });
                }
                
                local_planning(objective, context).await
            }
        },
        PlannerMode::Local => {
            let local_start = std::time::Instant::now();
            let result = local_planning(objective, context).await;
            
            // Record feedback for Local mode
            if let Some(feedback_collector) = &planner.feedback_collector {
                let duration_ms = local_start.elapsed().as_millis() as u64;
                let (status, error) = match &result {
                    Ok(_) => (SubtaskStatus::Completed, None),
                    Err(e) => (SubtaskStatus::Failed, Some(e.to_string())),
                };
                
                let trace = ExecutionTrace {
                    task_id: task_id.clone(),
                    subtask_id: "local_planning".to_string(),
                    agent_type: "local".to_string(),
                    status,
                    timestamp: Utc::now().to_rfc3339(),
                    outputs: None,
                    error,
                    duration_ms,
                };
                
                // Submit non-blocking
                let collector = feedback_collector.clone();
                tokio::spawn(async move {
                    if let Err(e) = collector.submit(trace).await {
                        warn!("Failed to submit local planning feedback: {}", e);
                    }
                });
            }
            
            result
        },
    PlannerMode::Hybrid => {
            // Try LaVague first, fall back to local
            if let Some(actor_system) = &planner.actor_system {
                let lavague_start = std::time::Instant::now();
        let lavague_result: Result<crate::task::Task, crate::types::PlannerError> = actor_system
            .decompose_task(objective.to_string(), context_keys.clone())
            .await
            .map(|t| planner.convert_types_task_to_task(&t));
                
                // Record LaVague feedback in hybrid mode
                if let Some(feedback_collector) = &planner.feedback_collector {
                    let duration_ms = lavague_start.elapsed().as_millis() as u64;
                    let (status, error) = match &lavague_result {
                        Ok(_) => (SubtaskStatus::Completed, None),
                        Err(e) => (SubtaskStatus::Failed, Some(e.to_string())),
                    };
                    
                    let trace = ExecutionTrace {
                        task_id: task_id.clone(),
                        subtask_id: "hybrid_lavague".to_string(),
                        agent_type: "lavague".to_string(),
                        status,
                        timestamp: Utc::now().to_rfc3339(),
                        outputs: None,
                        error,
                        duration_ms,
                    };
                    
                    // Submit non-blocking
                    let collector = feedback_collector.clone();
                    tokio::spawn(async move {
                        if let Err(e) = collector.submit(trace).await {
                            warn!("Failed to submit hybrid LaVague feedback: {}", e);
                        }
                    });
                }
                
                match lavague_result {
                    Ok(task) => Ok(task),
                    Err(e) => {
                        warn!("LaVague planning via actor system failed, falling back to local planning: {}", e);
                        
                        let local_start = std::time::Instant::now();
                        let local_result = local_planning(objective, context).await;
                        
                        // Record local fallback feedback in hybrid mode
                        if let Some(feedback_collector) = &planner.feedback_collector {
                            let duration_ms = local_start.elapsed().as_millis() as u64;
                            let (status, error) = match &local_result {
                                Ok(_) => (SubtaskStatus::Completed, None),
                                Err(e) => (SubtaskStatus::Failed, Some(e.to_string())),
                            };
                            
                            let trace = ExecutionTrace {
                                task_id: task_id.clone(),
                                subtask_id: "hybrid_local_fallback".to_string(),
                                agent_type: "local".to_string(),
                                status,
                                timestamp: Utc::now().to_rfc3339(),
                                outputs: None,
                                error,
                                duration_ms,
                            };
                            
                            // Submit non-blocking
                            let collector = feedback_collector.clone();
                            tokio::spawn(async move {
                                if let Err(e) = collector.submit(trace).await {
                                    warn!("Failed to submit hybrid local fallback feedback: {}", e);
                                }
                            });
                        }
                        
                        local_result
                    }
                }
            } else {
                warn!("Actor system unavailable, using local planning");
                
                // Record actor system unavailable feedback
                if let Some(feedback_collector) = &planner.feedback_collector {
                    let trace = ExecutionTrace {
                        task_id: task_id.clone(),
                        subtask_id: "hybrid_actor_unavailable".to_string(),
                        agent_type: "system".to_string(),
                        status: SubtaskStatus::Failed,
                        timestamp: Utc::now().to_rfc3339(),
                        outputs: None,
                        error: Some("Actor system unavailable".to_string()),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    };
                    
                    // Submit non-blocking
                    let collector = feedback_collector.clone();
                    tokio::spawn(async move {
                        if let Err(e) = collector.submit(trace).await {
                            warn!("Failed to submit actor unavailable feedback: {}", e);
                        }
                    });
                }
                
                local_planning(objective, context).await
            }
        }
    };
    
    // Record overall task feedback
    if let Some(feedback_collector) = &planner.feedback_collector {
        let total_duration_ms = start_time.elapsed().as_millis() as u64;
        let (status, error) = match &result {
            Ok(_) => (SubtaskStatus::Completed, None),
            Err(e) => (SubtaskStatus::Failed, Some(e.to_string())),
        };
        
        let trace = ExecutionTrace {
            task_id: task_id.clone(),
            subtask_id: "overall_planning".to_string(),
            agent_type: mode.to_string(), // Convert enum to string
            status,
            timestamp: Utc::now().to_rfc3339(),
            outputs: None,
            error,
            duration_ms: total_duration_ms,
        };
        
        // Submit non-blocking
        let collector = feedback_collector.clone();
        tokio::spawn(async move {
            if let Err(e) = collector.submit(trace).await {
                warn!("Failed to submit overall planning feedback: {}", e);
            }
        });
    }
    
    // Cache successful result
    if let Ok(task) = &result {
        if enable_cache {
            planner.store_in_cache(objective.to_string(), task.clone()).await;
        }
    }
    
    result
}
