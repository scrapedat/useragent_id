use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MemoryValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Json(serde_json::Value),
    List(Vec<MemoryValue>),
}

#[derive(Clone)]
pub struct SharedContext {
    data: Arc<Mutex<HashMap<String, MemoryValue>>>,
}

impl SharedContext {
    pub fn new() -> Self {
        Self { data: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn set(&self, key: impl Into<String>, value: MemoryValue) {
        let mut guard = self.data.lock().unwrap();
        guard.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<MemoryValue> {
        let guard = self.data.lock().unwrap();
        guard.get(key).cloned()
    }

    pub fn keys(&self) -> Vec<String> {
        let guard = self.data.lock().unwrap();
        guard.keys().cloned().collect()
    }

    pub fn has(&self, key: &str) -> bool {
        let guard = self.data.lock().unwrap();
        guard.contains_key(key)
    }
}

// Convenience constructors
impl MemoryValue {
    pub fn string(s: impl Into<String>) -> Self { MemoryValue::String(s.into()) }
    pub fn number(n: f64) -> Self { MemoryValue::Number(n) }
    pub fn boolean(b: bool) -> Self { MemoryValue::Boolean(b) }
    pub fn json(j: serde_json::Value) -> Self { MemoryValue::Json(j) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_context_basic() {
        let ctx = SharedContext::new();
        assert!(!ctx.has("a"));
        ctx.set("a", MemoryValue::number(1.0));
        assert!(ctx.has("a"));
        assert!(matches!(ctx.get("a"), Some(MemoryValue::Number(n)) if (n-1.0).abs() < 1e-6));
        let keys = ctx.keys();
        assert_eq!(keys, vec!["a".to_string()]);
    }
}
