use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct AgentModule {
    memory: Vec<u8>,
    patterns: Vec<Pattern>,
}

#[derive(Serialize, Deserialize)]
struct Pattern {
    sequence: Vec<String>,
    frequency: usize,
    confidence: f32,
}

#[wasm_bindgen]
impl AgentModule {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            memory: Vec::new(),
            patterns: Vec::new(),
        }
    }

    #[wasm_bindgen]
    pub fn detect_patterns(&mut self, session_data: &str) -> Result<String, JsValue> {
        // Parse session data
        let session: SessionState = serde_json::from_str(session_data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Analyze patterns
        let patterns = self.analyze_patterns(&session.events);
        
        // Return JSON string
        serde_json::to_string(&patterns)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn suggest_automation(&mut self, context: &str) -> Result<String, JsValue> {
        // Analyze context and suggest automations
        let suggestions = vec![
            "Automate repetitive clicks",
            "Create keyboard shortcuts",
            "Record and replay sequence",
        ];

        serde_json::to_string(&suggestions)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn execute_task(&mut self, task_id: &str) -> bool {
        // Execute automation task
        log(&format!("Executing task: {}", task_id));
        true
    }

    // Private helper methods
    fn analyze_patterns(&self, events: &[DOMEvent]) -> Vec<Pattern> {
        let mut patterns = Vec::new();
        let mut current_sequence = Vec::new();
        
        for event in events {
            current_sequence.push(format!("{:?}", event));
            
            if current_sequence.len() >= 3 {
                // Look for repeated sequences
                if self.is_repeated_sequence(&current_sequence) {
                    patterns.push(Pattern {
                        sequence: current_sequence.clone(),
                        frequency: self.count_sequence_frequency(&current_sequence, events),
                        confidence: 0.8,
                    });
                }
                
                current_sequence.remove(0);
            }
        }
        
        patterns
    }

    fn is_repeated_sequence(&self, sequence: &[String]) -> bool {
        sequence.windows(2).all(|w| w[0] == w[1])
    }

    fn count_sequence_frequency(&self, sequence: &[String], events: &[DOMEvent]) -> usize {
        let seq_str = sequence.join(",");
        let events_str: Vec<String> = events.iter()
            .map(|e| format!("{:?}", e))
            .collect();
        let events_str = events_str.join(",");
        
        events_str.matches(&seq_str).count()
    }
}
