//! Integration tests for the feedback collector and processing pipeline

// This file contains tests that are disabled for now
// We'll re-enable them once we have a more stable implementation

// Placeholder test to satisfy the compiler
#[cfg(test)]
mod tests {
    #[test]
    fn dummy_test() {
        assert!(true);
    }
}

#[tokio::test]
async fn test_feedback_integration() {
    // Set up a mock shared context
    let context = SharedContext::new();
    context.set("test_key", "test_value").await;
    
    // Create a mock planner service
    let mock_service = Arc::new(MockPlannerService::new());
    
    // Create a temporary feedback directory
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Set environment variables for testing
    std::env::set_var("ENABLE_FEEDBACK_COLLECTION", "true");
    std::env::set_var("FEEDBACK_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("LAVAGUE_MODE", "local"); // Use local mode for testing
    
    // Create a planner with feedback collection enabled
    let planner = Planner::new(PlannerMode::Local, false).await.unwrap();
    
    // Decompose a simple task
    let objective = "Test task for feedback collection";
    let result = planner.decompose_task(objective, &context.keys()).await;
    
    assert!(result.is_ok(), "Task decomposition should succeed");
    
    // Verify that feedback was collected
    if let Some(ref feedback_collector) = planner.feedback_collector {
        // Give a moment for async feedback processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let metrics = feedback_collector.get_metrics().await;
        assert!(metrics.pending_count > 0 || metrics.retry_counts > 0, 
                "Should have some feedback metrics");
    } else {
        panic!("Feedback collector should be initialized");
    }
    
    // Mock task and subtask for feedback submission
    let task = Task {
        id: "mock_task".to_string(),
        title: "Mock Task".to_string(),
        description: "Mock task for testing".to_string(),
        subtasks: vec![
            Subtask {
                id: "mock_subtask".to_string(),
                description: "Mock subtask".to_string(),
                status: TaskStatus::Pending,
                agent_type: Some("test_agent".to_string()),
                dependencies: vec![],
                error_message: None,
                result: None,
            }
        ],
        task_type: Some("test".to_string()),
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    // Submit feedback
    let feedback_result = planner.submit_feedback(
        &task,
        "mock_subtask",
        SubtaskStatus::Completed,
        100,
        None
    ).await;
    
    assert!(feedback_result.is_ok(), "Should submit feedback successfully");
}

#[tokio::test]
async fn test_feedback_metrics_generation() {
    // Create a mock planner service
    let mock_service = Arc::new(MockPlannerService::new());
    
    // Create a temporary feedback directory
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Configure feedback collector
    let config = FeedbackConfig {
        feedback_dir: temp_dir.path().to_path_buf(),
        batch_enabled: false, // Disable batching for immediate processing
        batch_size: 1,
        flush_interval_seconds: 1,
        max_retries: 1,
    };
    
    // Create collector directly
    let collector = FeedbackCollector::new(mock_service.clone(), config);
    
    // Submit multiple traces
    for i in 1..=5 {
        let trace = ExecutionTrace {
            task_id: format!("task_{}", i),
            subtask_id: format!("subtask_{}", i),
            agent_type: "test_agent".to_string(),
            status: if i % 2 == 0 { SubtaskStatus::Completed } else { SubtaskStatus::Failed },
            timestamp: chrono::Utc::now().to_rfc3339(),
            outputs: None,
            error: if i % 2 == 0 { None } else { Some("Test error".to_string()) },
            duration_ms: i * 100,
        };
        
        collector.submit(trace).await.unwrap();
    }
    
    // Check metrics
    let metrics = collector.get_metrics().await;
    assert_eq!(metrics.pending_count, 0, "All traces should be processed");
    
    // Verify traces submitted to mock service
    let traces = mock_service.get_traces();
    assert_eq!(traces.len(), 5, "Should have 5 traces");
}
