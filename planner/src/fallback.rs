use crate::task::{Task, Subtask, AgentType, TaskStatus};
use crate::capability::{AgentCapability, CapabilityDiscovery};
use crate::types::PlannerError;
use memory::SharedContext;
use anyhow::{Result, Context as AnyhowContext};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;
use log::{info, warn, debug};
use serde::{Serialize, Deserialize};

/// Strategy rule for creating plans based on input patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningRule {
    /// Name of the rule
    pub name: String,
    
    /// Priority (lower number = higher priority)
    pub priority: u32,
    
    /// Keywords that trigger this rule
    pub keywords: Vec<String>,
    
    /// Agent type needed for this rule
    pub agent_type: String,
    
    /// Required input keys
    pub required_inputs: Vec<String>,
    
    /// Output keys produced
    pub output_keys: Vec<String>,
    
    /// Stage in a multi-step workflow (lower = earlier)
    pub stage: u32,
}

/// Rule-based fallback planner
pub struct FallbackPlanner {
    /// Rules for planning
    rules: Vec<PlanningRule>,
    
    /// Capability discovery service
    capability_discovery: Arc<CapabilityDiscovery>,
    
    /// Learned patterns (objective -> successful plan)
    learned_patterns: HashMap<String, Task>,
}

impl FallbackPlanner {
    /// Create a new fallback planner
    pub fn new(capability_discovery: Arc<CapabilityDiscovery>) -> Self {
        Self {
            rules: Self::default_rules(),
            capability_discovery,
            learned_patterns: HashMap::new(),
        }
    }
    
    /// Load additional rules from a file
    pub async fn load_rules<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let rules: Vec<PlanningRule> = serde_json::from_str(&content)?;
        
        self.rules.extend(rules);
        // Sort by priority
        self.rules.sort_by_key(|r| r.priority);
        
        Ok(())
    }
    
    /// Default set of planning rules
    fn default_rules() -> Vec<PlanningRule> {
        vec![
            // Scraping rule
            PlanningRule {
                name: "Web Scraping".to_string(),
                priority: 10,
                keywords: vec![
                    "scrape".to_string(),
                    "browse".to_string(), 
                    "visit".to_string(), 
                    "navigate".to_string(),
                    "search".to_string(),
                    "find online".to_string(),
                    "look up".to_string(),
                    "website".to_string(),
                    "webpage".to_string(),
                    "http".to_string(),
                    "https".to_string(),
                    "url".to_string(),
                ],
                agent_type: "Scrape".to_string(),
                required_inputs: vec!["target_url".to_string()],
                output_keys: vec!["page_content".to_string(), "status_code".to_string()],
                stage: 10,
            },
            
            // Processing rule
            PlanningRule {
                name: "Content Processing".to_string(),
                priority: 20,
                keywords: vec![
                    "process".to_string(),
                    "analyze".to_string(), 
                    "extract".to_string(), 
                    "summarize".to_string(),
                    "parse".to_string(),
                    "transform".to_string(),
                ],
                agent_type: "Process".to_string(),
                required_inputs: vec!["content".to_string(), "page_content".to_string()],
                output_keys: vec!["processed_data".to_string(), "summary".to_string()],
                stage: 20,
            },
            
            // Storage rule
            PlanningRule {
                name: "Data Storage".to_string(),
                priority: 30,
                keywords: vec![
                    "save".to_string(),
                    "store".to_string(), 
                    "record".to_string(), 
                    "persist".to_string(),
                    "database".to_string(),
                ],
                agent_type: "Data".to_string(),
                required_inputs: vec!["data".to_string(), "processed_data".to_string(), "content".to_string()],
                output_keys: vec!["stored_location".to_string(), "success".to_string()],
                stage: 30,
            },
            
            // Vision analysis rule
            PlanningRule {
                name: "Vision Analysis".to_string(),
                priority: 15,
                keywords: vec![
                    "image".to_string(),
                    "picture".to_string(), 
                    "photo".to_string(), 
                    "visual".to_string(),
                    "vision".to_string(),
                    "look at".to_string(),
                    "analyze image".to_string(),
                ],
                agent_type: "Vision".to_string(),
                required_inputs: vec!["image_path".to_string(), "image_url".to_string()],
                output_keys: vec!["vision_result".to_string(), "detected_objects".to_string()],
                stage: 15,
            },
            
            // Time-based rule
            PlanningRule {
                name: "Time Operations".to_string(),
                priority: 25,
                keywords: vec![
                    "schedule".to_string(),
                    "timer".to_string(), 
                    "wait".to_string(), 
                    "delay".to_string(),
                    "remind".to_string(),
                    "after".to_string(),
                    "before".to_string(),
                ],
                agent_type: "Time".to_string(),
                required_inputs: vec!["duration".to_string(), "target_time".to_string()],
                output_keys: vec!["completion_time".to_string()],
                stage: 25,
            },
        ]
    }
    
    /// Generate a plan based on the objective and available context
    pub async fn generate_plan(&self, objective: &str, context: &SharedContext) -> Result<Task> {
        info!("Generating fallback plan for objective: {}", objective);
        
        // Check if we have a learned pattern for this or similar objective
        if let Some(task) = self.find_similar_learned_pattern(objective) {
            info!("Using learned pattern for objective: {}", objective);
            return Ok(task);
        }
        
        // Discover available capabilities
        let capabilities = self.capability_discovery.get_all_capabilities().await;
        debug!("Available capabilities: {:?}", capabilities.iter().map(|c| &c.name).collect::<Vec<_>>());
        
        // Extract context keys
        let context_keys = context.keys();
        debug!("Available context keys: {:?}", context_keys);
        
        // Match rules based on objective
        let mut matched_rules = self.match_rules(objective);
        debug!("Matched rules: {:?}", matched_rules.iter().map(|r| &r.name).collect::<Vec<_>>());
        
        // Verify capabilities are available for matched rules
        matched_rules = self.filter_rules_by_capabilities(&matched_rules, &capabilities);
        debug!("Rules after capability filtering: {:?}", matched_rules.iter().map(|r| &r.name).collect::<Vec<_>>());
        
        // Generate a dependency graph
        let dependency_graph = self.build_dependency_graph(&matched_rules, &context_keys);
        debug!("Dependency graph built with {} nodes", dependency_graph.len());
        
        // Generate subtasks from the graph
        let subtasks = self.generate_subtasks(objective, &dependency_graph, &context_keys)?;
        debug!("Generated {} subtasks", subtasks.len());
        
        // Create the final task
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let task_id = Uuid::new_v4().to_string();
        
        let task = Task { 
            id: task_id, 
            objective: objective.to_string(), 
            subtasks, 
            status: TaskStatus::Pending, 
            created_at: now,
        };
        
        Ok(task)
    }
    
    /// Find a similar learned pattern based on objective similarity
    fn find_similar_learned_pattern(&self, objective: &str) -> Option<Task> {
        // In a real implementation, this would use semantic similarity
        // For now, use simple keyword matching
        for (known_objective, task) in &self.learned_patterns {
            // Check for substring match (very simple heuristic)
            if known_objective.contains(objective) || objective.contains(known_objective) {
                let mut task_copy = task.clone();
                // Update the objective to match the current one
                task_copy.objective = objective.to_string();
                return Some(task_copy);
            }
        }
        
        None
    }
    
    /// Match rules based on the objective
    fn match_rules(&self, objective: &str) -> Vec<&PlanningRule> {
        let lower_obj = objective.to_lowercase();
        
        let mut matched_rules = Vec::new();
        
        for rule in &self.rules {
            // Check if any keyword matches
            if rule.keywords.iter().any(|k| lower_obj.contains(&k.to_lowercase())) {
                matched_rules.push(rule);
            }
        }
        
        // If no rules matched, use a general purpose rule
        if matched_rules.is_empty() {
            // Return all rules that don't require specific inputs
            matched_rules.extend(self.rules.iter().filter(|r| r.required_inputs.is_empty()));
        }
        
        // Sort by stage for proper ordering
        matched_rules.sort_by_key(|r| r.stage);
        
        matched_rules
    }
    
    /// Filter rules by available capabilities
    fn filter_rules_by_capabilities<'a>(&self, rules: &[&'a PlanningRule], capabilities: &[AgentCapability]) -> Vec<&'a PlanningRule> {
        let available_agent_types: HashSet<String> = capabilities.iter()
            .map(|c| c.agent_type.to_string())
            .collect();
            
        rules.iter()
            .filter(|r| available_agent_types.contains(&r.agent_type))
            .cloned()
            .collect()
    }
    
    /// Build a dependency graph based on matched rules and available context
    fn build_dependency_graph<'a>(&self, rules: &[&'a PlanningRule], context_keys: &[String]) -> Vec<(&'a PlanningRule, Vec<&'a PlanningRule>)> {
        let mut graph = Vec::new();
        
        // First, identify rules that can be executed with available context
        let mut available_keys: HashSet<String> = context_keys.iter().cloned().collect();
        let mut executable_rules = Vec::new();
        
        for rule in rules {
            if rule.required_inputs.iter().all(|k| available_keys.contains(k)) || rule.required_inputs.is_empty() {
                executable_rules.push(*rule);
                // Add outputs to available keys
                for output in &rule.output_keys {
                    available_keys.insert(output.clone());
                }
            }
        }
        
        // Build dependency graph
        for rule in rules {
            let dependencies = rules.iter()
                .filter(|r| r.stage < rule.stage && r.output_keys.iter().any(|k| rule.required_inputs.contains(k)))
                .cloned()
                .collect();
                
            graph.push((*rule, dependencies));
        }
        
        graph
    }
    
    /// Generate subtasks from the dependency graph
    fn generate_subtasks(&self, objective: &str, graph: &[(&PlanningRule, Vec<&PlanningRule>)], context_keys: &[String]) -> Result<Vec<Subtask>> {
        let mut subtasks = Vec::new();
        let mut id_map = HashMap::new();
        
        // First pass: create subtasks without dependencies
        for (rule, _) in graph {
            let subtask_id = Uuid::new_v4().to_string();
            id_map.insert(rule.name.clone(), subtask_id.clone());
            
            // Determine inputs based on context and rule requirements
            let mut input_keys = Vec::new();
            for required in &rule.required_inputs {
                if context_keys.contains(required) {
                    input_keys.push(required.clone());
                }
            }
            
            // Create the subtask
            let agent_type = match rule.agent_type.as_str() {
                "Scrape" => AgentType::Scrape,
                "Process" => AgentType::Process,
                "Vision" => AgentType::Vision,
                "Time" => AgentType::Time,
                "Data" => AgentType::Data,
                _ => AgentType::Custom(rule.agent_type.clone()),
            };
            
            let subtask = Subtask {
                id: subtask_id,
                objective: format!("{}: {}", rule.name, objective),
                required_agent: agent_type,
                dependencies: Vec::new(), // Will be filled in second pass
                input_keys,
                output_key: if !rule.output_keys.is_empty() {
                    rule.output_keys[0].clone()
                } else {
                    "result".to_string()
                },
            };
            
            subtasks.push(subtask);
        }
        
        // Second pass: add dependencies
        for (i, (rule, dependencies)) in graph.iter().enumerate() {
            let subtask_id = id_map.get(&rule.name).unwrap();
            
            // Find the subtask
            if let Some(subtask) = subtasks.iter_mut().find(|s| &s.id == subtask_id) {
                // Add dependencies
                for dep in dependencies {
                    if let Some(dep_id) = id_map.get(&dep.name) {
                        subtask.dependencies.push(dep_id.clone());
                    }
                }
            }
        }
        
        Ok(subtasks)
    }
    
    /// Learn from a successful execution
    pub fn learn_from_execution(&mut self, objective: String, task: Task) {
        // Store the successful plan for future use
        self.learned_patterns.insert(objective, task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_fallback_planner() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();
        
        // Create capability discovery
        let discovery = Arc::new(CapabilityDiscovery::new(base_dir));
        
        // Create fallback planner
        let planner = FallbackPlanner::new(discovery);
        
        // Create a context
        let mut context = SharedContext::new();
        context.insert("target_url".to_string(), "https://example.com".to_string());
        
        // Generate a plan
        let result = planner.generate_plan("Scrape example.com", &context).await;
        
        // It should succeed even without capabilities since we're not enforcing them in tests
        assert!(result.is_ok(), "Failed to generate plan: {:?}", result.err());
        
        let task = result.unwrap();
        assert!(!task.subtasks.is_empty(), "Task should have subtasks");
    }
    
    #[tokio::test]
    async fn test_rule_matching() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();
        
        // Create capability discovery
        let discovery = Arc::new(CapabilityDiscovery::new(base_dir));
        
        // Create fallback planner
        let planner = FallbackPlanner::new(discovery);
        
        // Test rule matching
        let rules = planner.match_rules("Scrape website data");
        assert!(!rules.is_empty(), "Should match at least one rule");
        assert_eq!(rules[0].name, "Web Scraping", "Should match scraping rule");
        
        let rules = planner.match_rules("Process and analyze text");
        assert!(!rules.is_empty(), "Should match at least one rule");
        assert_eq!(rules[0].name, "Content Processing", "Should match processing rule");
        
        let rules = planner.match_rules("Save data to database");
        assert!(!rules.is_empty(), "Should match at least one rule");
        assert_eq!(rules[0].name, "Data Storage", "Should match storage rule");
        
        // Generic query should still return rules
        let rules = planner.match_rules("Do something");
        assert!(!rules.is_empty(), "Should return fallback rules");
    }
}
