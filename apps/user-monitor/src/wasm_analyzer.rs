use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use super::BehaviorAnalyzer;
use std::time::Instant;

#[wasm_bindgen]
pub struct WasmAnalyzer {
    analyzer: BehaviorAnalyzer,
}

#[wasm_bindgen]
impl WasmAnalyzer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            analyzer: BehaviorAnalyzer::new(),
        }
    }

    #[wasm_bindgen]
    pub fn process_action(&mut self, action: &str) -> Result<String, JsValue> {
        self.analyzer.analyze_action(action, Instant::now());
        
        // Get predictions
        if let Some(intent) = self.analyzer.predict_next_action() {
            serde_json::to_string(&intent)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Ok("{}".to_string())
        }
    }

    #[wasm_bindgen]
    pub fn get_patterns(&self) -> Result<String, JsValue> {
        let patterns = self.analyzer.get_patterns();
        serde_json::to_string(&patterns)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}
