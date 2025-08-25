use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUnderstanding {
    pub task_id: String,
    pub description: String,
    pub steps: Vec<TaskStep>,
    pub implicit_rules: Vec<ImplicitRule>,
    pub required_skills: HashSet<String>,
    pub data_requirements: Vec<DataRequirement>,
    pub decision_points: Vec<DecisionPoint>,
    pub validation_rules: Vec<ValidationRule>,
    pub dependencies: HashMap<String, String>,
    pub estimated_complexity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: String,
    pub description: String,
    pub step_type: StepType,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub prerequisites: Vec<String>,
    pub validation: Option<String>,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    Navigation { url_pattern: String },
    DataExtraction { selectors: Vec<String> },
    Interaction { action: InteractionType },
    Validation { rules: Vec<String> },
    Decision { conditions: Vec<String> },
    DataTransformation { operations: Vec<String> },
    Notification { channels: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Click { selector: String },
    Input { selector: String, value_type: String },
    Select { selector: String, options: Vec<String> },
    Scroll { target: String },
    Wait { condition: String },
    Custom { action: String, params: HashMap<String, String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitRule {
    pub rule_type: RuleType,
    pub description: String,
    pub examples: Vec<String>,
    pub confidence: f32,
    pub context: HashMap<String, String>,
}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    VisualDiscrepancy,
    TextualDiscrepancy,
    PriceAnomaly,
    TimingSensitive,
    ContextDependent,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirement {
    pub skill_type: SkillType,
    pub importance: f32,
    pub training_data_needed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillType {
    ImageAnalysis,
    TextComparison,
    PriceAnalysis,
    MarketKnowledge,
    WebNavigation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_type: SourceType,
    pub url: String,
    pub required_fields: Vec<String>,
    pub extraction_rules: Vec<ExtractionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    WebPage,
    API,
    Image,
    PDF,
    Database,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    pub field: String,
    pub selector: String,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    pub criterion_type: CriterionType,
    pub threshold: f32,
    pub measurement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CriterionType {
    ProfitMargin,
    ResponseTime,
    AccuracyRate,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClarification {
    pub question: String,
    pub context: String,
    pub expected_insight: String,
}

pub struct TaskAnalyzer {
    pub understanding: TaskUnderstanding,
    clarification_needed: Vec<TaskClarification>,
    training_requirements: HashMap<String, Vec<String>>,
}

impl TaskAnalyzer {
    pub fn new(description: &str) -> Self {
        Self {
            understanding: TaskUnderstanding {
                task_id: uuid::Uuid::new_v4().to_string(),
                description: description.to_string(),
                implicit_rules: Vec::new(),
                required_skills: Vec::new(),
                data_sources: Vec::new(),
                success_criteria: Vec::new(),
            },
            clarification_needed: Vec::new(),
            training_requirements: HashMap::new(),
        }
    }

    pub fn analyze_task(&mut self) -> Result<Vec<TaskClarification>> {
        // Identify unclear aspects that need user clarification
        self.identify_implicit_knowledge();
        self.determine_required_skills();
        self.analyze_data_requirements();
        Ok(self.clarification_needed.clone())
    }

    fn identify_implicit_knowledge(&mut self) {
        // Look for keywords indicating unstated knowledge
        let implicit_indicators = [
            "good deal", "worth", "valuable", "interesting",
            "better than", "potential", "opportunity",
        ];

        for indicator in implicit_indicators {
            if self.understanding.description.contains(indicator) {
                self.clarification_needed.push(TaskClarification {
                    question: format!("Could you define what makes a {} in this context?", indicator),
                    context: format!("Found term '{}' which needs quantifiable criteria", indicator),
                    expected_insight: "Specific metrics or criteria for decision making".to_string(),
                });
            }
        }
    }

    fn determine_required_skills(&mut self) {
        // Check for visual analysis needs
        if self.understanding.description.contains("image") 
            || self.understanding.description.contains("picture")
            || self.understanding.description.contains("photo") {
            self.understanding.required_skills.push(SkillRequirement {
                skill_type: SkillType::ImageAnalysis,
                importance: 0.9,
                training_data_needed: true,
            });
            
            self.training_requirements.insert(
                "vision_model".to_string(),
                vec![
                    "image classification".to_string(),
                    "object detection".to_string(),
                    "visual anomaly detection".to_string(),
                ]
            );
        }

        // Check for text analysis needs
        if self.understanding.description.contains("description")
            || self.understanding.description.contains("text")
            || self.understanding.description.contains("listing") {
            self.understanding.required_skills.push(SkillRequirement {
                skill_type: SkillType::TextComparison,
                importance: 0.8,
                training_data_needed: true,
            });
            
            self.training_requirements.insert(
                "text_model".to_string(),
                vec![
                    "text classification".to_string(),
                    "semantic comparison".to_string(),
                    "named entity recognition".to_string(),
                ]
            );
        }
    }

    fn analyze_data_requirements(&mut self) {
        // Identify needed data sources
        if self.understanding.description.contains("auction") {
            self.understanding.data_sources.push(DataSource {
                source_type: SourceType::WebPage,
                url: "auction_site".to_string(),
                required_fields: vec!["title".to_string(), "description".to_string(), "images".to_string(), "price".to_string()],
                extraction_rules: vec![
                    ExtractionRule {
                        field: "images".to_string(),
                        selector: "img.listing-image".to_string(),
                        validation: Some("min-count:1".to_string()),
                    }
                ],
            });
        }

        // Add comparison data sources
        if self.understanding.description.contains("compare") {
            self.understanding.data_sources.push(DataSource {
                source_type: SourceType::API,
                url: "ebay_completed_listings".to_string(),
                required_fields: vec!["title".to_string(), "final_price".to_string(), "condition".to_string()],
                extraction_rules: vec![],
            });
        }
    }
}
