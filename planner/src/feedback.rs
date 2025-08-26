use crate::task::{Task, Subtask, TaskStatus};
use crate::types::{ExecutionTrace, PlannerError, SubtaskStatus};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use anyhow::Result;
use log::{info, warn, error, debug};

/// Configuration for the feedback collector
#[derive(Debug, Clone)]
pub struct FeedbackConfig {
    /// Directory to store feedback files
    pub feedback_dir: PathBuf,
    
    /// Whether to batch feedback submissions
    pub batch_enabled: bool,
    
    /// Maximum batch size
    pub batch_size: usize,
    
    /// Batch flush interval in seconds
    pub flush_interval_seconds: u64,
    
    /// Number of retries for failed submissions
    pub max_retries: u32,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            feedback_dir: PathBuf::from("./feedback"),
            batch_enabled: true,
            batch_size: 10,
            flush_interval_seconds: 60,
            max_retries: 3,
        }
    }
}

/// Execution feedback collector and processor
pub struct FeedbackCollector {
    /// Configuration
    config: FeedbackConfig,
    
    /// Feedback submission channel
    submission_tx: mpsc::Sender<ExecutionTrace>,
    
    /// Pending feedback traces
    pending: Arc<RwLock<Vec<ExecutionTrace>>>,
    
    /// Submission retry counts
    retry_counts: Arc<RwLock<HashMap<String, u32>>>,
    
    /// Failed traces that exceeded retry limit
    failed: Arc<RwLock<Vec<ExecutionTrace>>>,
    
    /// Service client for submitting feedback
    client: Arc<dyn crate::client::PlannerService + Send + Sync>,
}

impl FeedbackCollector {
    /// Create a new feedback collector
    pub fn new(client: Arc<dyn crate::client::PlannerService + Send + Sync>, config: FeedbackConfig) -> Self {
        let (submission_tx, submission_rx) = mpsc::channel(100);
        let pending = Arc::new(RwLock::new(Vec::new()));
        let retry_counts = Arc::new(RwLock::new(HashMap::new()));
        let failed = Arc::new(RwLock::new(Vec::new()));
        
        let collector = Self {
            config,
            submission_tx,
            pending: pending.clone(),
            retry_counts: retry_counts.clone(),
            failed: failed.clone(),
            client: client.clone(),
        };
        
        // Spawn background processor
        if collector.config.batch_enabled {
            let client = client.clone();
            let pending = pending.clone();
            let retry_counts = retry_counts.clone();
            let failed = failed.clone();
            let config = collector.config.clone();
            
            tokio::spawn(async move {
                Self::process_feedback_queue(
                    submission_rx,
                    client,
                    pending,
                    retry_counts,
                    failed,
                    config,
                ).await;
            });
        }
        
        collector
    }
    
    /// Submit execution feedback
    pub async fn submit(&self, trace: ExecutionTrace) -> Result<()> {
        if self.config.batch_enabled {
            // Submit to queue for batched processing
            self.submission_tx.send(trace).await
                .map_err(|e| anyhow::anyhow!("Failed to queue feedback: {}", e))?;
            Ok(())
        } else {
            // Submit immediately
            self.submit_trace(trace).await
        }
    }
    
    /// Submit a trace directly to the service
    async fn submit_trace(&self, trace: ExecutionTrace) -> Result<()> {
        match self.client.submit_feedback(&trace).await {
            Ok(_) => {
                debug!("Successfully submitted feedback for task {} subtask {}", 
                       trace.task_id, trace.subtask_id);
                Ok(())
            },
            Err(e) => {
                warn!("Failed to submit feedback for task {} subtask {}: {}",
                      trace.task_id, trace.subtask_id, e);
                
                // Store for retry if needed
                if self.config.batch_enabled {
                    self.pending.write().await.push(trace);
                }
                
                Err(anyhow::anyhow!("Failed to submit feedback: {}", e))
            }
        }
    }
    
    /// Background queue processor
    async fn process_feedback_queue(
        mut submission_rx: mpsc::Receiver<ExecutionTrace>,
        client: Arc<dyn crate::client::PlannerService + Send + Sync>,
        pending: Arc<RwLock<Vec<ExecutionTrace>>>,
        retry_counts: Arc<RwLock<HashMap<String, u32>>>,
        failed: Arc<RwLock<Vec<ExecutionTrace>>>,
        config: FeedbackConfig,
    ) {
        // Create the feedback directory if it doesn't exist
        if !config.feedback_dir.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&config.feedback_dir).await {
                error!("Failed to create feedback directory: {}", e);
            }
        }
        
        // Set up flush interval
        let mut flush_interval = interval(Duration::from_secs(config.flush_interval_seconds));
        
        loop {
            tokio::select! {
                // Process incoming traces
                trace = submission_rx.recv() => {
                    if let Some(trace) = trace {
                        // Add to pending queue
                        pending.write().await.push(trace);
                        
                        // Flush if we hit batch size
                        if pending.read().await.len() >= config.batch_size {
                            Self::flush_pending_traces(
                                &client,
                                &pending,
                                &retry_counts,
                                &failed,
                                &config,
                            ).await;
                        }
                    } else {
                        // Channel closed, exit loop
                        break;
                    }
                }
                
                // Process periodic flush
                _ = flush_interval.tick() => {
                    Self::flush_pending_traces(
                        &client,
                        &pending,
                        &retry_counts,
                        &failed,
                        &config,
                    ).await;
                }
            }
        }
    }
    
    /// Flush pending traces to the service
    async fn flush_pending_traces(
        client: &Arc<dyn crate::client::PlannerService + Send + Sync>,
        pending: &Arc<RwLock<Vec<ExecutionTrace>>>,
        retry_counts: &Arc<RwLock<HashMap<String, u32>>>,
        failed: &Arc<RwLock<Vec<ExecutionTrace>>>,
        config: &FeedbackConfig,
    ) {
        // Get pending traces
        let traces = {
            let mut pending = pending.write().await;
            std::mem::take(&mut *pending)
        };
        
        if traces.is_empty() {
            return;
        }
        
        info!("Submitting batch of {} execution traces", traces.len());
        
        // Process each trace
        for trace in traces {
            let trace_id = format!("{}-{}", trace.task_id, trace.subtask_id);
            
            // Submit to service
            match client.submit_feedback(&trace).await {
                Ok(_) => {
                    debug!("Successfully submitted feedback for trace {}", trace_id);
                    
                    // Remove from retry counts if it was there
                    retry_counts.write().await.remove(&trace_id);
                }
                Err(e) => {
                    warn!("Failed to submit feedback for trace {}: {}", trace_id, e);
                    
                    // Update retry count
                    let mut retry_counts = retry_counts.write().await;
                    let count = *retry_counts.entry(trace_id.clone()).or_insert(0) + 1;
                    retry_counts.insert(trace_id.clone(), count);
                    
                    // Check if we should retry
                    if count < config.max_retries {
                        // Add back to pending
                        pending.write().await.push(trace.clone());
                    } else {
                        // Exceeded retry limit, move to failed
                        warn!("Exceeded retry limit for trace {}, moving to failed", trace_id);
                        failed.write().await.push(trace.clone());
                        
                        // Save to disk for later recovery
                        let filename = format!("failed_trace_{}.json", trace_id);
                        let path = config.feedback_dir.join(filename);
                        
                        if let Ok(json) = serde_json::to_string_pretty(&trace) {
                            if let Err(e) = tokio::fs::write(&path, json).await {
                                error!("Failed to write failed trace to disk: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get metrics for the feedback collector
    pub async fn get_metrics(&self) -> FeedbackMetrics {
        FeedbackMetrics {
            pending_count: self.pending.read().await.len(),
            failed_count: self.failed.read().await.len(),
            retry_counts: self.retry_counts.read().await.len(),
        }
    }
    
    /// Attempt to resubmit failed traces
    pub async fn resubmit_failed(&self) -> Result<usize> {
        // Get failed traces
        let traces = {
            let mut failed = self.failed.write().await;
            std::mem::take(&mut *failed)
        };
        
        if traces.is_empty() {
            return Ok(0);
        }
        
        info!("Attempting to resubmit {} failed traces", traces.len());
        
        let mut success_count = 0;
        
        // Process each trace
        for trace in traces {
            // Submit to service
            match self.client.submit_feedback(&trace).await {
                Ok(_) => {
                    debug!("Successfully resubmitted feedback for task {} subtask {}", 
                           trace.task_id, trace.subtask_id);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to resubmit feedback for task {} subtask {}: {}",
                          trace.task_id, trace.subtask_id, e);
                    
                    // Add back to failed
                    self.failed.write().await.push(trace);
                }
            }
        }
        
        Ok(success_count)
    }
    
    /// Load failed traces from disk
    pub async fn load_failed_from_disk(&self) -> Result<usize> {
        // Check if directory exists
        if !self.config.feedback_dir.exists() {
            return Ok(0);
        }
        
        // Read directory
        let mut entries = tokio::fs::read_dir(&self.config.feedback_dir).await?;
        let mut count = 0;
        
        // Process each file
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Skip non-JSON files
            if let Some(ext) = path.extension() {
                if ext != "json" {
                    continue;
                }
            } else {
                continue;
            }
            
            // Read file
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    // Parse trace
                    match serde_json::from_str::<ExecutionTrace>(&content) {
                        Ok(trace) => {
                            // Add to failed
                            self.failed.write().await.push(trace);
                            count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to parse trace from file {:?}: {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read file {:?}: {}", path, e);
                }
            }
        }
        
        Ok(count)
    }
}

/// Metrics for the feedback collector
#[derive(Debug, Clone)]
pub struct FeedbackMetrics {
    /// Number of pending traces
    pub pending_count: usize,
    
    /// Number of failed traces
    pub failed_count: usize,
    
    /// Number of traces being retried
    pub retry_counts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExecutionTrace, SubtaskStatus};
    
    // Mock planner service for testing
    #[derive(Default)]
    struct MockPlannerService {
        traces: std::sync::Mutex<Vec<ExecutionTrace>>,
        error_mode: std::sync::Mutex<bool>,
    }
    
    impl MockPlannerService {
        fn new() -> Self {
            Self {
                traces: std::sync::Mutex::new(Vec::new()),
                error_mode: std::sync::Mutex::new(false),
            }
        }
        
        fn set_error_mode(&self, error_mode: bool) {
            *self.error_mode.lock().unwrap() = error_mode;
        }
        
        fn get_traces(&self) -> Vec<ExecutionTrace> {
            self.traces.lock().unwrap().clone()
        }
    }
    
    #[async_trait::async_trait]
    impl crate::client::PlannerService for MockPlannerService {
        async fn decompose_task(&self, _objective: &str, _context: &[String]) -> Result<crate::task::Task, crate::types::PlannerError> {
            Err(crate::types::PlannerError::Other("Not implemented".to_string()))
        }
        
        async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), crate::types::PlannerError> {
            if *self.error_mode.lock().unwrap() {
                Err(crate::types::PlannerError::ApiError("Test error".to_string()))
            } else {
                self.traces.lock().unwrap().push(trace.clone());
                Ok(())
            }
        }
    }
    
    #[tokio::test]
    async fn test_feedback_collector() {
        // Create a mock service
        let mock_service = Arc::new(MockPlannerService::new());
        
        // Create a temporary directory for feedback
        let temp_dir = tempdir().unwrap();
        
        // Create config
        let config = FeedbackConfig {
            feedback_dir: temp_dir.path().to_path_buf(),
            batch_enabled: true,
            batch_size: 2,
            flush_interval_seconds: 1,
            max_retries: 2,
        };
        
        // Create collector
        let collector = FeedbackCollector::new(mock_service.clone(), config);
        
        // Create a test trace
        let trace = ExecutionTrace {
            task_id: "task_1".to_string(),
            subtask_id: "sub_1".to_string(),
            agent_type: "Test".to_string(),
            status: SubtaskStatus::Completed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            outputs: None,
            error: None,
            duration_ms: 100,
        };
        
        // Submit trace
        let result = collector.submit(trace.clone()).await;
        assert!(result.is_ok(), "Failed to submit feedback: {:?}", result);
        
        // Wait for flush (might be immediate if batch_size is 1)
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check metrics
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.pending_count, 0, "Should have no pending traces");
        
        // Check that the trace was submitted to the mock service
        let traces = mock_service.get_traces();
        assert_eq!(traces.len(), 1, "Should have one trace");
        assert_eq!(traces[0].task_id, "task_1", "Should have correct task ID");
    }
    
    #[tokio::test]
    async fn test_feedback_batching() {
        // Create a mock service
        let mock_service = Arc::new(MockPlannerService::new());
        
        // Create a temporary directory for feedback
        let temp_dir = tempdir().unwrap();
        
        // Create config with larger batch size
        let config = FeedbackConfig {
            feedback_dir: temp_dir.path().to_path_buf(),
            batch_enabled: true,
            batch_size: 3,
            flush_interval_seconds: 1,
            max_retries: 2,
        };
        
        // Create collector
        let collector = FeedbackCollector::new(mock_service.clone(), config);
        
        // Submit 2 traces (less than batch size)
        for i in 1..=2 {
            let trace = ExecutionTrace {
                task_id: format!("task_{}", i),
                subtask_id: format!("sub_{}", i),
                agent_type: "Test".to_string(),
                status: SubtaskStatus::Completed,
                timestamp: chrono::Utc::now().to_rfc3339(),
                outputs: None,
                error: None,
                duration_ms: 100,
            };
            
            collector.submit(trace).await.unwrap();
        }
        
        // Should be pending until flush interval or batch size is reached
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.pending_count, 2, "Should have 2 pending traces");
        
        // Submit one more to trigger batch flush
        let trace = ExecutionTrace {
            task_id: "task_3".to_string(),
            subtask_id: "sub_3".to_string(),
            agent_type: "Test".to_string(),
            status: SubtaskStatus::Completed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            outputs: None,
            error: None,
            duration_ms: 100,
        };
        
        collector.submit(trace).await.unwrap();
        
        // Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check that all traces were submitted
        let traces = mock_service.get_traces();
        assert_eq!(traces.len(), 3, "Should have submitted 3 traces");
        
        // Check metrics
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.pending_count, 0, "Should have no pending traces");
    }
    
    #[tokio::test]
    async fn test_feedback_retry() {
        // Create a failing mock service
        let mut mock_service = MockPlannerService::new();
        mock_service.set_error_mode(true);
        let mock_service = Arc::new(mock_service);
        
        // Create a temporary directory for feedback
        let temp_dir = tempdir().unwrap();
        
        // Create config with short interval
        let config = FeedbackConfig {
            feedback_dir: temp_dir.path().to_path_buf(),
            batch_enabled: true,
            batch_size: 1,
            flush_interval_seconds: 1,
            max_retries: 2,
        };
        
        // Create collector
        let collector = FeedbackCollector::new(mock_service.clone(), config);
        
        // Submit a trace
        let trace = ExecutionTrace {
            task_id: "task_1".to_string(),
            subtask_id: "sub_1".to_string(),
            agent_type: "Test".to_string(),
            status: SubtaskStatus::Completed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            outputs: None,
            error: None,
            duration_ms: 100,
        };
        
        collector.submit(trace).await.unwrap();
        
        // Wait for retry attempts
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Check metrics - should be in failed after max_retries
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.pending_count, 0, "Should have no pending traces");
        assert_eq!(metrics.failed_count, 1, "Should have one failed trace");
        
        // Check that a file was written
        let files = std::fs::read_dir(&temp_dir).unwrap().count();
        assert_eq!(files, 1, "Should have one failure file");
    }
}
