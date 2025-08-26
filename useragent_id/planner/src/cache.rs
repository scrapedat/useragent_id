use std::collections::HashMap;
use crate::types::Task;

/// Cache for storing plans to avoid redundant calls to LaVague
pub struct PlanCache {
    capacity: usize,
    cache: HashMap<String, (Task, chrono::DateTime<chrono::Utc>)>,
    // Keys in insertion order for LRU eviction
    keys_order: Vec<String>,
}

impl PlanCache {
    /// Create a new plan cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            cache: HashMap::with_capacity(capacity),
            keys_order: Vec::with_capacity(capacity),
        }
    }
    
    /// Get a task from the cache if it exists
    pub fn get(&self, objective: &str) -> Option<Task> {
        self.cache.get(objective).map(|(task, _)| task.clone())
    }
    
    /// Insert a task into the cache
    pub fn insert(&mut self, objective: String, task: Task) {
        // If the cache is full, remove the oldest entry
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&objective) {
            if let Some(old_key) = self.keys_order.first().cloned() {
                self.cache.remove(&old_key);
                self.keys_order.remove(0);
            }
        }
        
        // Insert the new entry
        let now = chrono::Utc::now();
        
        // Update keys order if needed
        if let Some(idx) = self.keys_order.iter().position(|k| k == &objective) {
            self.keys_order.remove(idx);
        }
        self.keys_order.push(objective.clone());
        
        self.cache.insert(objective, (task, now));
    }
    
    /// Clear entries older than the given duration
    pub fn clear_old_entries(&mut self, max_age: chrono::Duration) {
        let now = chrono::Utc::now();
        let old_keys: Vec<String> = self.cache
            .iter()
            .filter_map(|(k, (_, t))| {
                if now - *t > max_age {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        
        for key in &old_keys {
            self.cache.remove(key);
            if let Some(idx) = self.keys_order.iter().position(|k| k == key) {
                self.keys_order.remove(idx);
            }
        }
    }
    
    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.keys_order.clear();
    }
    
    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    
    /// Get metrics for the cache
    pub fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            size: self.cache.len(),
            capacity: self.capacity,
            oldest_entry: self.keys_order.first().and_then(|k| {
                self.cache.get(k).map(|(_, t)| *t)
            }),
            newest_entry: self.keys_order.last().and_then(|k| {
                self.cache.get(k).map(|(_, t)| *t)
            }),
        }
    }
}

/// Metrics for the plan cache
pub struct CacheMetrics {
    /// Current number of entries
    pub size: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Timestamp of the oldest entry
    pub oldest_entry: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of the newest entry
    pub newest_entry: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Task, TaskMetadata, Subtask, SubtaskStatus};
    
    fn create_test_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            objective: format!("Test task {}", id),
            subtasks: vec![
                Subtask {
                    id: format!("{}_sub1", id),
                    objective: format!("Subtask for {}", id),
                    required_agent: "Test".to_string(),
                    input_keys: vec![],
                    output_keys: vec![],
                    status: SubtaskStatus::Pending,
                    dependencies: vec![],
                }
            ],
            metadata: TaskMetadata {
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                planner: Some("test".to_string()),
                cached: true,
                version: Some("test".to_string()),
            },
        }
    }
    
    #[test]
    fn test_cache_insertion_and_retrieval() {
        let mut cache = PlanCache::new(2);
        let task1 = create_test_task("1");
        let task2 = create_test_task("2");
        
        cache.insert("objective1".to_string(), task1.clone());
        cache.insert("objective2".to_string(), task2.clone());
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("objective1").unwrap().id, "1");
        assert_eq!(cache.get("objective2").unwrap().id, "2");
    }
    
    #[test]
    fn test_cache_eviction() {
        let mut cache = PlanCache::new(2);
        let task1 = create_test_task("1");
        let task2 = create_test_task("2");
        let task3 = create_test_task("3");
        
        cache.insert("objective1".to_string(), task1);
        cache.insert("objective2".to_string(), task2);
        cache.insert("objective3".to_string(), task3);
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("objective1"), None);  // Evicted
        assert_eq!(cache.get("objective2").unwrap().id, "2");
        assert_eq!(cache.get("objective3").unwrap().id, "3");
    }
    
    #[test]
    fn test_clear_old_entries() {
        let mut cache = PlanCache::new(3);
        let task1 = create_test_task("1");
        let task2 = create_test_task("2");
        
        cache.insert("objective1".to_string(), task1);
        
        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        cache.insert("objective2".to_string(), task2);
        
        cache.clear_old_entries(chrono::Duration::milliseconds(50));
        
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("objective1"), None);  // Cleared
        assert_eq!(cache.get("objective2").unwrap().id, "2");
    }
}
