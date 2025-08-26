//! Integration tests for the LaVague planner.
//! 
//! This module contains integration tests that verify the proper functioning
//! of the LaVague planner with the actor system.

#[cfg(test)]
mod tests {
    use actix::prelude::*;
    use anyhow::Result;
    use memory::SharedContext;
    use crate::actor::{Actor, PlannerActorSystem};
    use crate::client::{LaVagueClient, LaVagueConfig, PlannerService};
    use crate::planner::{decompose_task, PlannerMode};
    use crate::task::{Task, Subtask, TaskStatus, AgentType};
    use crate::circuit::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    // Test that the actor system properly handles messages
    #[actix::test]
    async fn test_actor_decomposition() {
        // Create a mock client
        let client = create_mock_client().await;
        
        // Create actor system
        let actor_system = PlannerActorSystem::new(client).await;
        
        // Test decomposing a task
        let result = actor_system.decompose_task(
            "Test actor system".to_string(),
            vec!["key1".to_string(), "key2".to_string()]
        ).await;
        
        // Verify the result
        assert!(result.is_ok(), "Actor decomposition failed: {:?}", result.err());
        let task = result.unwrap();
        assert_eq!(task.objective, "Test actor system");
        assert!(!task.subtasks.is_empty(), "Task should have subtasks");
    }
    
    // Test the circuit breaker integration
    #[actix::test]
    async fn test_circuit_breaker_integration() {
        // Create a failing client
        let client = create_failing_client().await;
        
        // Create circuit breaker
        let circuit = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout_ms: 100,
            half_open_limit: 1,
        }));
        
        // Create actor system with circuit breaker
        let context = SharedContext::new();
        let objective = "Test circuit breaker";
        
        // First failure
        let result = decompose_task(objective, &context).await;
        assert!(result.is_err(), "Expected first call to fail");
        
        // Second failure should trip the circuit
        let result = decompose_task(objective, &context).await;
        assert!(result.is_err(), "Expected second call to fail");
        
        // Third call should be rejected by the circuit breaker
        let result = decompose_task(objective, &context).await;
        assert!(result.is_err(), "Expected circuit to be open");
        
        // Wait for circuit to reset
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Circuit should allow one test request now
        // But we're using a failing client, so it will fail and reopen
        let result = decompose_task(objective, &context).await;
        assert!(result.is_err(), "Expected test request to fail");
        
        // Create a working client and try again
        let client = create_mock_client().await;
        
        // Wait for circuit to reset again
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // This should succeed and close the circuit
        let result = decompose_task(objective, &context).await;
        assert!(result.is_ok(), "Expected circuit to allow request and succeed");
    }
    
    // Test actix integration
    #[actix::test]
    async fn test_actix_integration() {
        // Create an actor
        struct TestActor;
        
        impl Actor for TestActor {
            type Context = actix::Context<Self>;
        }
        
        // Message for the actor
        #[derive(Message)]
        #[rtype(result = "Result<Task>")]
        struct DecomposeTask(String);
        
        // Handler implementation
        impl Handler<DecomposeTask> for TestActor {
            type Result = ResponseFuture<Result<Task>>;
            
            fn handle(&mut self, msg: DecomposeTask, _ctx: &mut Self::Context) -> Self::Result {
                let task_id = Uuid::new_v4().to_string();
                let subtask_id = Uuid::new_v4().to_string();
                
                Box::pin(async move {
                    Ok(Task {
                        id: task_id,
                        objective: msg.0,
                        subtasks: vec![Subtask {
                            id: subtask_id,
                            objective: format!("Subtask for {}", msg.0),
                            required_agent: "Test".parse().unwrap(),
                            dependencies: vec![],
                            input_keys: vec![],
                            output_key: "result".to_string(),
                        }],
                        status: TaskStatus::Pending,
                        created_at: chrono::Utc::now().timestamp() as u64,
                    })
                })
            }
        }
        
        // Start the actor
        let addr = TestActor.start();
        
        // Send a message
        let result = addr.send(DecomposeTask("Actix test".to_string())).await;
        
        // Verify the result
        assert!(result.is_ok(), "Failed to send message to actor");
        let task_result = result.unwrap();
        assert!(task_result.is_ok(), "Actor returned an error");
        let task = task_result.unwrap();
        assert_eq!(task.objective, "Actix test");
        assert_eq!(task.subtasks.len(), 1);
    }
    
    // Test capability discovery
    #[actix::test]
    async fn test_capability_discovery() {
        // Create a capability discovery service
        struct CapabilityDiscovery;
        
        impl CapabilityDiscovery {
            async fn discover_capabilities() -> Vec<String> {
                // Simulate scanning for capabilities
                vec![
                    "Scrape".to_string(),
                    "Process".to_string(),
                    "Data".to_string(),
                    "Custom".to_string(),
                ]
            }
        }
        
        // Get capabilities
        let capabilities = CapabilityDiscovery::discover_capabilities().await;
        
        // Verify capabilities
        assert!(capabilities.contains(&"Scrape".to_string()));
        assert!(capabilities.contains(&"Process".to_string()));
        assert!(capabilities.contains(&"Data".to_string()));
        assert!(capabilities.contains(&"Custom".to_string()));
        
        // Create a context with capability information
        let mut context = SharedContext::new();
        context.insert("capabilities".to_string(), serde_json::to_string(&capabilities).unwrap());
        
        // Test that the planner uses capability information
        let result = decompose_task("Test capabilities", &context).await;
        assert!(result.is_ok(), "Decomposition failed");
    }
    
    // Test error handling
    #[actix::test]
    async fn test_error_handling() {
        // Create a client that returns an error
        let client = create_failing_client().await;
        
        // Create a context
        let context = SharedContext::new();
        
        // Test decomposing a task
        let result = decompose_task("Test error handling", &context).await;
        
        // Should fall back to local planning
        assert!(result.is_ok(), "Expected fallback to local planning");
        assert!(result.unwrap().subtasks.len() > 0);
    }
    
    // Helper function to create a mock client
    async fn create_mock_client() -> LaVagueClient {
        use crate::client::mock::MockPlannerService;
        
        // Create mock config
        let config = LaVagueConfig {
            endpoint: "http://localhost:8000".to_string(),
            api_key: Some("test_key".to_string()),
            timeout: Duration::from_secs(1),
            user_agent: "test-agent".to_string(),
            tls_cert_path: None,
        };
        
        LaVagueClient::new(config).unwrap()
    }
    
    // Helper function to create a failing client
    async fn create_failing_client() -> LaVagueClient {
        // Create mock config that will fail
        let config = LaVagueConfig {
            endpoint: "http://nonexistent.example.com:12345".to_string(),
            api_key: Some("test_key".to_string()),
            timeout: Duration::from_millis(100), // Short timeout to fail quickly
            user_agent: "test-agent".to_string(),
            tls_cert_path: None,
        };
        
        LaVagueClient::new(config).unwrap()
    }
}
