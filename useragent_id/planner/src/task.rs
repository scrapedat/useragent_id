use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub objective: String,
    pub subtasks: Vec<Subtask>,
    pub status: TaskStatus,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Subtask {
    pub id: String,
    pub objective: String,
    pub required_agent: AgentType,
    pub dependencies: Vec<String>,
    pub input_keys: Vec<String>,
    pub output_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TaskStatus { Pending, InProgress, Completed, Failed }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgentType { Scrape, Vision, Time, Data, Process, Custom(String) }

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Scrape => write!(f, "Scrape"),
            AgentType::Vision => write!(f, "Vision"),
            AgentType::Time => write!(f, "Time"),
            AgentType::Data => write!(f, "Data"),
            AgentType::Process => write!(f, "Process"),
            AgentType::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Scrape" => Ok(AgentType::Scrape),
            "Vision" => Ok(AgentType::Vision),
            "Time" => Ok(AgentType::Time),
            "Data" => Ok(AgentType::Data),
            "Process" => Ok(AgentType::Process),
            s => Ok(AgentType::Custom(s.to_string())),
        }
    }
}
