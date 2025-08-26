//! Integration tests for LaVague planner
//! 
//! These tests verify that the LaVague planner integration works correctly
//! and handles edge cases appropriately.

#[cfg(test)]
mod tests {
    use crate::client::{LaVagueClient, LaVagueConfig, mock::MockPlannerService};
    use crate::actor::PlannerActorSystem;
    use crate::types::{Task, ExecutionTrace, SubtaskStatus};
    use crate::planner::PlannerMode;
    use std::time::Duration;
    use tokio::sync::oneshot;
    use async_trait::async_trait;

    /// Test that the mock planner service works
    #[tokio::test]
    async fn test_mock_planner() {
        let mock = MockPlannerService::new();
        let task = mock.decompose_task("Test objective", &[]).await.unwrap();
        
        assert_eq!(task.objective, "Test objective");
        assert!(!task.subtasks.is_empty());
    }
    
    /// Test that the actor system properly handles messages
    #[tokio::test]
    async fn test_actor_system() {
        let mock = MockPlannerService::new();
        let client = LaVagueClient::new(LaVagueConfig::default()).unwrap();
        let actor_system = PlannerActorSystem::new(client).await;
        
        let objective = "Test actor system".to_string();
        let context_keys = vec!["key1".to_string(), "key2".to_string()];
        
        let result = actor_system.decompose_task(objective.clone(), context_keys.clone()).await;
        
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.objective, objective);
    }
    
    /// Test error handling in the planner
    #[tokio::test]
    async fn test_error_handling() {
        let mut mock = MockPlannerService::new();
        mock.set_error_mode(true);
        
        let result = mock.decompose_task("This should fail", &[]).await;
        assert!(result.is_err());
    }
    
    /// Test the feedback submission
    #[tokio::test]
    async fn test_feedback_submission() {
        let mock = MockPlannerService::new();
        
        let trace = ExecutionTrace {
            task_id: "task_1".to_string(),
            subtask_id: "sub_1".to_string(),
            agent_type: "Scrape".to_string(),
            status: SubtaskStatus::Completed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            outputs: Some(serde_json::json!({ "result": "Success" })),
            error: None,
            duration_ms: 1000,
        };
        
        let result = mock.submit_feedback(&trace).await;
        assert!(result.is_ok());
    }
    
    /// Test timeout handling
    #[tokio::test]
    async fn test_timeout_handling() {
        struct SlowMockService;
        
        #[async_trait]
        impl crate::client::PlannerService for SlowMockService {
            async fn decompose_task(&self, _objective: &str, _context: &[String]) -> Result<Task, crate::types::PlannerError> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Err(crate::types::PlannerError::Timeout)
            }
            
            async fn submit_feedback(&self, _trace: &ExecutionTrace) -> Result<(), crate::types::PlannerError> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Err(crate::types::PlannerError::Timeout)
            }
        }
        
        let service = SlowMockService;
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            service.decompose_task("Test timeout", &[])
        ).await;
        
        assert!(result.is_err()); // Should timeout
    }
    
    /// Test channel handling
    #[tokio::test]
    async fn test_channel_handling() {
        let (tx, rx) = oneshot::channel();
        
        // Drop the receiver to simulate a closed channel
        drop(rx);
        
        // Sending should not panic
        let result = tx.send("test");
        assert!(result.is_err());
    }
    
    /// Test proper cleanup of resources
    #[tokio::test]
    async fn test_resource_cleanup() {
        // Create resources that should be cleaned up
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("audit.log");
        
        // Create audit logger
        let logger = crate::security::AuditLogger::new(path.clone()).unwrap();
        
        // Log something
        logger.log("test", &serde_json::json!({ "test": true })).unwrap();
        
        // Verify log exists
        assert!(path.exists());
        
        // Clean up
        drop(logger);
        temp_dir.close().unwrap();
    }
    
    /// Test data validation
    #[tokio::test]
    async fn test_data_validation() {
        use crate::security::DataSanitizer;
        
        // Test objective validation
        assert!(DataSanitizer::validate_objective("Valid objective").is_ok());
        assert!(DataSanitizer::validate_objective("").is_err()); // Empty
        assert!(DataSanitizer::validate_objective(&"x".repeat(2000)).is_err()); // Too long
        
        // Test context keys validation
        assert!(DataSanitizer::validate_context_keys(&["key1".to_string(), "key2".to_string()]).is_ok());
        assert!(DataSanitizer::validate_context_keys(&[]).is_ok()); // Empty list is ok
        
        // Test string sanitization
        let sanitized = DataSanitizer::sanitize_string("Valid\nwith\nnewlines\tand\ttabs");
        assert_eq!(sanitized, "Valid\nwith\nnewlines\tand\ttabs");
        
        let sanitized = DataSanitizer::sanitize_string("Invalid\x00with\x01control\x02chars");
        assert_eq!(sanitized, "Invalidwithcontrolchars");
    }
    
    /// Test backoff strategies
    #[tokio::test]
    async fn test_backoff_strategies() {
        use crate::security::BackoffStrategy;
        
        // Fixed backoff
        let fixed = BackoffStrategy::Fixed { delay: 1000 };
        assert_eq!(fixed.calculate_backoff(1).as_millis(), 1000);
        assert_eq!(fixed.calculate_backoff(5).as_millis(), 1000);
        
        // Exponential backoff
        let exp = BackoffStrategy::Exponential {
            initial: 1000,
            multiplier: 2.0,
            max: 8000,
        };
        assert_eq!(exp.calculate_backoff(0).as_millis(), 1000); // 1000 * 2^0
        assert_eq!(exp.calculate_backoff(1).as_millis(), 2000); // 1000 * 2^1
        assert_eq!(exp.calculate_backoff(2).as_millis(), 4000); // 1000 * 2^2
        assert_eq!(exp.calculate_backoff(3).as_millis(), 8000); // 1000 * 2^3
        assert_eq!(exp.calculate_backoff(4).as_millis(), 8000); // Should cap at max
        
        // Fibonacci backoff
        let fib = BackoffStrategy::Fibonacci {
            initial: 1000,
            max: 10000,
        };
        assert_eq!(fib.calculate_backoff(0).as_millis(), 1000); // First
        assert_eq!(fib.calculate_backoff(1).as_millis(), 1000); // Second
        assert_eq!(fib.calculate_backoff(2).as_millis(), 2000); // Third
        assert_eq!(fib.calculate_backoff(3).as_millis(), 3000); // Fourth
        assert_eq!(fib.calculate_backoff(4).as_millis(), 5000); // Fifth
        assert_eq!(fib.calculate_backoff(5).as_millis(), 8000); // Sixth
        assert_eq!(fib.calculate_backoff(6).as_millis(), 10000); // Seventh (capped)
    }
}
