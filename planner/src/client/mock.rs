//! Mock implementation for testing

use crate::task::{Task, Subtask, TaskStatus};
use crate::types::{ExecutionTrace, PlannerError};
use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

/// Mock implementation of the PlannerService for testing
pub struct MockPlannerService {
    traces: Mutex<Vec<ExecutionTrace>>,
    should_error: Mutex<bool>,
}

impl MockPlannerService {
    /// Create a new mock service
    pub fn new() -> Self {
        Self {
            traces: Mutex::new(Vec::new()),
            should_error: Mutex::new(false),
        }
    }
    
    /// Set whether the mock should return errors
    pub fn set_error_mode(&mut self, should_error: bool) {
        let mut error = self.should_error.lock().unwrap();
        *error = should_error;
    }
    
    /// Get all submitted traces
    pub fn get_traces(&self) -> Vec<ExecutionTrace> {
        let traces = self.traces.lock().unwrap();
        traces.clone()
    }
    
    /// Clear all traces
    pub fn clear_traces(&self) {
        let mut traces = self.traces.lock().unwrap();
        traces.clear();
    }
}

#[async_trait]
impl super::PlannerService for MockPlannerService {
    async fn decompose_task(&self, objective: &str, _context: &[String]) -> Result<Task, PlannerError> {
        // Check if we should return an error
        if *self.should_error.lock().unwrap() {
            return Err(PlannerError::ServiceError("Mock service error".to_string()));
        }
        
        // Create a simple mock task
        let task_id = Uuid::new_v4().to_string();
        
        let subtasks = vec![
            Subtask {
                id: Uuid::new_v4().to_string(),
                description: format!("Mock subtask 1 for {}", objective),
                status: TaskStatus::Pending,
                dependencies: vec![],
                agent_type: Some("scraper".to_string()),
                error_message: None,
                result: None,
            },
            Subtask {
                id: Uuid::new_v4().to_string(),
                description: format!("Mock subtask 2 for {}", objective),
                status: TaskStatus::Pending,
                dependencies: vec![],
                agent_type: Some("analyzer".to_string()),
                error_message: None,
                result: None,
            },
        ];
        
        let task = Task {
            id: task_id,
            title: format!("Mock task for: {}", objective),
            description: objective.to_string(),
            subtasks,
            task_type: Some("test".to_string()),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        Ok(task)
    }
    
    async fn submit_feedback(&self, trace: &ExecutionTrace) -> Result<(), PlannerError> {
        // Check if we should return an error
        if *self.should_error.lock().unwrap() {
            return Err(PlannerError::ServiceError("Mock service error".to_string()));
        }
        
        // Store the trace
        let mut traces = self.traces.lock().unwrap();
        traces.push(trace.clone());
        
        Ok(())
    }
}
