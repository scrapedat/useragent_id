use serde::{Deserialize, Serialize};
use memory::SharedContext;
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExecutionStatus { Success, PartialSuccess(f32), Failure(String) }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionTrace {
    pub trace_id: String,
    pub task_id: String,
    pub agent_type: String,
    pub input: String,
    pub output: String,
    pub expected_output: Option<String>,
    pub status: ExecutionStatus,
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

pub struct Evaluator { pub context: SharedContext }

impl Evaluator {
    pub fn new(context: SharedContext) -> Self { Self { context } }

    pub fn record_success(&self, task_id: &str, agent: &str, input: &str, output: &str) -> ExecutionTrace {
        self.create_trace(task_id, agent, input, output, ExecutionStatus::Success)
    }

    pub fn record_failure(&self, task_id: &str, agent: &str, input: &str, output: &str, error: &str) -> ExecutionTrace {
        self.create_trace(task_id, agent, input, output, ExecutionStatus::Failure(error.to_string()))
    }

    fn create_trace(&self, task_id: &str, agent: &str, input: &str, output: &str, status: ExecutionStatus) -> ExecutionTrace {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let trace_id = format!("trace_{}_{}", task_id, now);
        ExecutionTrace { trace_id, task_id: task_id.to_string(), agent_type: agent.to_string(), input: input.to_string(), output: output.to_string(), expected_output: None, status, timestamp: now, metadata: serde_json::json!({}) }
    }

    pub fn save_trace(&self, trace: &ExecutionTrace) -> anyhow::Result<()> {
        std::fs::create_dir_all("traces")?;
        let path = format!("traces/{}.json", trace.trace_id);
        std::fs::write(path, serde_json::to_string_pretty(trace)?)?;
        Ok(())
    }
}
