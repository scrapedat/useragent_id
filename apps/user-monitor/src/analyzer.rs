use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};

// Small neural network for pattern recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TinyNN {
    weights: Vec<f32>,
    bias: f32,
}

impl TinyNN {
    fn new(input_size: usize) -> Self {
        Self {
            weights: vec![0.1; input_size],
            bias: 0.0,
        }
    }

    fn predict(&self, inputs: &[f32]) -> f32 {
        let sum: f32 = inputs.iter()
            .zip(self.weights.iter())
            .map(|(x, w)| x * w)
            .sum();
        (sum + self.bias).tanh()
    }

    fn train(&mut self, inputs: &[f32], target: f32, learning_rate: f32) {
        let prediction = self.predict(inputs);
        let error = target - prediction;
        
        // Update weights and bias
        for (w, x) in self.weights.iter_mut().zip(inputs.iter()) {
            *w += learning_rate * error * x;
        }
        self.bias += learning_rate * error;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorPattern {
    pub sequence: Vec<String>,
    pub frequency: usize,
    pub average_duration: Duration,
    pub confidence: f32,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntent {
    pub action_type: String,
    pub target: String,
    pub confidence: f32,
    pub context: HashMap<String, String>,
}

#[derive(Debug)]
pub struct BehaviorAnalyzer {
    patterns: HashMap<String, UserBehaviorPattern>,
    recent_actions: VecDeque<(String, Instant)>,
    neural_net: TinyNN,
    window_size: usize,
    min_pattern_length: usize,
}

impl BehaviorAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            recent_actions: VecDeque::with_capacity(100),
            neural_net: TinyNN::new(10), // 10 features per action
            window_size: 5,
            min_pattern_length: 3,
        }
    }

    pub fn analyze_action(&mut self, action: &str, timestamp: Instant) {
        // Add action to recent history
        self.recent_actions.push_back((action.to_string(), timestamp));
        if self.recent_actions.len() > self.window_size {
            self.recent_actions.pop_front();
        }

        // Extract features from recent actions
        let features = self.extract_features();
        
        // Predict if this is part of a pattern
        let prediction = self.neural_net.predict(&features);
        
        if prediction > 0.7 {
            self.update_patterns();
        }
    }

    fn extract_features(&self) -> Vec<f32> {
        let mut features = vec![0.0; 10];
        
        if self.recent_actions.len() < 2 {
            return features;
        }

        // Feature 1: Time between actions
        let times: Vec<_> = self.recent_actions.iter()
            .map(|(_, t)| t)
            .collect();
        for i in 1..times.len() {
            let duration = times[i].duration_since(*times[i-1]);
            features[0] += duration.as_secs_f32();
        }
        features[0] /= (times.len() - 1) as f32;

        // Feature 2: Action repetition
        let actions: Vec<_> = self.recent_actions.iter()
            .map(|(a, _)| a)
            .collect();
        for i in 1..actions.len() {
            if actions[i] == actions[i-1] {
                features[1] += 1.0;
            }
        }
        features[1] /= (actions.len() - 1) as f32;

        // Feature 3-10: Action type frequencies
        let action_types = ["click", "key", "scroll", "move", "drag", "drop", "hover", "type"];
        for (i, action_type) in action_types.iter().enumerate() {
            features[i+2] = actions.iter()
                .filter(|a| a.contains(action_type))
                .count() as f32 / actions.len() as f32;
        }

        features
    }

    fn update_patterns(&mut self) {
        if self.recent_actions.len() < self.min_pattern_length {
            return;
        }

        // Create pattern key from recent actions
        let pattern_key: String = self.recent_actions.iter()
            .map(|(action, _)| action.as_str())
            .collect::<Vec<_>>()
            .join("->");

        // Calculate pattern duration
        let duration = self.recent_actions.back().unwrap().1
            .duration_since(self.recent_actions.front().unwrap().1);

        // Update or create pattern
        self.patterns.entry(pattern_key.clone())
            .and_modify(|p| {
                p.frequency += 1;
                p.average_duration = (p.average_duration + duration) / 2;
                p.confidence = (p.confidence + self.neural_net.predict(&self.extract_features())) / 2.0;
                p.last_seen = Utc::now();
            })
            .or_insert(UserBehaviorPattern {
                sequence: pattern_key.split("->").map(String::from).collect(),
                frequency: 1,
                average_duration: duration,
                confidence: self.neural_net.predict(&self.extract_features()),
                last_seen: Utc::now(),
            });
    }

    pub fn get_patterns(&self) -> Vec<&UserBehaviorPattern> {
        self.patterns.values()
            .filter(|p| p.confidence > 0.7)
            .collect()
    }

    pub fn predict_next_action(&self) -> Option<UserIntent> {
        if self.recent_actions.is_empty() {
            return None;
        }

        let features = self.extract_features();
        let prediction = self.neural_net.predict(&features);

        // Find most similar pattern
        let current_sequence: Vec<_> = self.recent_actions.iter()
            .map(|(action, _)| action.as_str())
            .collect();

        let mut best_match = None;
        let mut highest_similarity = 0.0;

        for pattern in self.patterns.values() {
            let similarity = self.sequence_similarity(&current_sequence, &pattern.sequence);
            if similarity > highest_similarity {
                highest_similarity = similarity;
                best_match = Some(pattern);
            }
        }

        best_match.map(|pattern| {
            let next_action = pattern.sequence.get(current_sequence.len())
                .unwrap_or(&pattern.sequence[0]);
            
            UserIntent {
                action_type: next_action.split('_').next()
                    .unwrap_or("unknown")
                    .to_string(),
                target: next_action.split('_').last()
                    .unwrap_or("unknown")
                    .to_string(),
                confidence: prediction * highest_similarity,
                context: HashMap::new(),
            }
        })
    }

    fn sequence_similarity(&self, seq1: &[&str], seq2: &[String]) -> f32 {
        let len = seq1.len().min(seq2.len());
        if len == 0 {
            return 0.0;
        }

        let matching = seq1.iter()
            .zip(seq2.iter())
            .filter(|(a, b)| *a == b.as_str())
            .count();

        matching as f32 / len as f32
    }
}

impl CodeAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;

        let rust_query = Query::new(
            tree_sitter_rust::language(),
            r#"
            (function_item
              name: (identifier) @function.name
              parameters: (parameters) @function.params
              return_type: (type_identifier)? @function.return_type
              body: (block) @function.body
            ) @function.def

            (line_comment) @comment
            (block_comment) @comment
            
            (call_expression
              function: [(identifier) (field_expression)] @function.call
            )

            (attribute_item) @attribute
            "#,
        )?;

        Ok(Self {
            parser,
            rust_query,
        })
    }

    pub fn analyze_code(&mut self, code: &str) -> Result<Vec<AnalyzedFunction>> {
        let tree = self.parser.parse(code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse code"))?;

        let mut functions = Vec::new();
        let mut cursor = QueryCursor::new();
        
        for m in cursor.matches(&self.rust_query, tree.root_node(), code.as_bytes()) {
            if let Some(func_def) = m.nodes_for_capture_index(0).next() {
                let mut analyzed = self.analyze_function_node(func_def, code)?;
                
                // Analyze complexity
                analyzed.complexity = self.calculate_complexity(func_def, code);
                
                // Find function calls
                analyzed.calls = self.find_function_calls(func_def, code);
                
                functions.push(analyzed);
            }
        }

        Ok(functions)
    }

    fn analyze_function_node(&self, node: tree_sitter::Node, code: &str) -> Result<AnalyzedFunction> {
        let name = self.get_node_text(node, "identifier", code)?;
        let params = self.extract_parameters(node, code)?;
        let return_type = self.get_return_type(node, code);
        let visibility = self.get_visibility(node, code);
        let documentation = self.extract_documentation(node, code);
        let async_status = self.is_async_function(node, code);

        Ok(AnalyzedFunction {
            name: name.to_string(),
            code: node.utf8_text(code.as_bytes())?.to_string(),
            complexity: 0, // Will be calculated later
            parameters: params,
            return_type,
            calls: vec![], // Will be filled later
            async_status,
            visibility,
            documentation,
        })
    }

    fn calculate_complexity(&self, node: tree_sitter::Node, code: &str) -> usize {
        let mut complexity = 1;
        let mut cursor = node.walk();
        
        // Traverse the syntax tree
        cursor.goto_first_child();
        loop {
            let node = cursor.node();
            match node.kind() {
                "if_expression" | "match_expression" | "for_expression" |
                "while_expression" | "loop_expression" | "closure_expression" |
                "binary_expression" | "await_expression" => {
                    complexity += 1;
                }
                _ => {}
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        
        complexity
    }

    fn find_function_calls(&self, node: tree_sitter::Node, code: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let mut cursor = node.walk();
        
        cursor.goto_first_child();
        loop {
            let node = cursor.node();
            if node.kind() == "call_expression" {
                if let Ok(call_name) = node.child_by_field_name("function")
                    .and_then(|n| Some(n.utf8_text(code.as_bytes()).ok()?.to_string())) {
                    calls.push(call_name);
                }
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        
        calls
    }

    fn get_node_text(&self, node: tree_sitter::Node, kind: &str, code: &str) -> Result<&str> {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        
        loop {
            let current = cursor.node();
            if current.kind() == kind {
                return Ok(current.utf8_text(code.as_bytes())?);
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        
        Err(anyhow::anyhow!("Node not found: {}", kind))
    }

    fn extract_parameters(&self, node: tree_sitter::Node, code: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        
        // Find parameters node
        cursor.goto_first_child();
        while cursor.node().kind() != "parameters" {
            if !cursor.goto_next_sibling() {
                return Ok(params);
            }
        }
        
        // Extract each parameter
        let params_node = cursor.node();
        cursor = params_node.walk();
        cursor.goto_first_child();
        
        loop {
            let current = cursor.node();
            if current.kind() == "parameter" {
                if let (Some(name), Some(type_info)) = (
                    current.child_by_field_name("pattern")
                        .and_then(|n| Some(n.utf8_text(code.as_bytes()).ok()?.to_string())),
                    current.child_by_field_name("type")
                        .and_then(|n| Some(n.utf8_text(code.as_bytes()).ok()?.to_string()))
                ) {
                    params.push(Parameter { name, type_info });
                }
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        
        Ok(params)
    }

    fn get_return_type(&self, node: tree_sitter::Node, code: &str) -> Option<String> {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        
        while cursor.goto_next_sibling() {
            let current = cursor.node();
            if current.kind() == "type_identifier" {
                return current.utf8_text(code.as_bytes()).ok().map(String::from);
            }
        }
        
        None
    }

    fn get_visibility(&self, node: tree_sitter::Node, code: &str) -> String {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        
        while cursor.goto_next_sibling() {
            let current = cursor.node();
            if current.kind() == "visibility_modifier" {
                return current.utf8_text(code.as_bytes())
                    .map(String::from)
                    .unwrap_or_else(|_| "private".to_string());
            }
        }
        
        "private".to_string()
    }

    fn extract_documentation(&self, node: tree_sitter::Node, code: &str) -> Option<String> {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        
        let mut docs = Vec::new();
        while cursor.goto_next_sibling() {
            let current = cursor.node();
            if current.kind() == "line_comment" || current.kind() == "block_comment" {
                if let Ok(comment) = current.utf8_text(code.as_bytes()) {
                    docs.push(comment.trim().to_string());
                }
            }
        }
        
        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n"))
        }
    }

    fn is_async_function(&self, node: tree_sitter::Node, code: &str) -> bool {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        
        while cursor.goto_next_sibling() {
            let current = cursor.node();
            if current.kind() == "async" {
                return true;
            }
        }
        
        false
    }
}
