use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ============================================================================
// Core Data Structures for the WasmAgentTrainer System
//
// This file defines the "language" that the different modules use to
// communicate. The data flows from a raw `RecordedEvent` to a structured
// `LearnedTask`, which is then compiled into a runnable `Agent`.
// ============================================================================


// ============================================================================
// Phase 1: Data from `user-monitor`
// Represents a single, raw event captured from the OS.
// ============================================================================

/// A single, raw event captured from the OS. This is the fundamental
/// unit of data for recording. It's designed to be serialized as a single
/// line in a JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    /// A unique ID for the session this event belongs to.
    pub session_id: Uuid,
    /// The timestamp of when the event occurred.
    pub timestamp: DateTime<Utc>,
    /// The actual event data from the OS.
    pub event_type: EventType,
}

/// Represents the different types of input events we can capture.
/// This is a simplified version of `rdev::EventType` for our purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    KeyPress(rdev::Key),
    KeyRelease(rdev::Key),
    ButtonPress(rdev::Button),
    ButtonRelease(rdev::Button),
    MouseMove { x: f64, y: f64 },
    /// Represents a line of text narrated by the user.
    Narration(String),
}

#[cfg(feature = "events")]
impl From<rdev::EventType> for EventType {
    fn from(event: rdev::EventType) -> Self {
        match event {
            rdev::EventType::KeyPress(key) => Self::KeyPress(key),
            rdev::EventType::KeyRelease(key) => Self::KeyRelease(key),
            rdev::EventType::ButtonPress(button) => Self::ButtonPress(button),
            rdev::EventType::ButtonRelease(button) => Self::ButtonRelease(button),
            rdev::EventType::MouseMove { x, y } => Self::MouseMove { x, y },
            // We can ignore other event types for now, but we need to return a valid EventType.
            // Using Narration with an empty string is a temporary workaround.
            _ => Self::Narration("[unhandled rdev event]".to_string()),
        }
    }
}


// ============================================================================
// Phase 2: Data from `task-learner`
// Represents a structured, cleaned-up, and understandable task.
// ============================================================================

/// A complete, understandable task learned from a recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedTask {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    /// The sequence of steps that make up the task.
    pub steps: Vec<AutomationStep>,
    /// The ID of the session this task was learned from.
    pub source_session_id: Uuid,
}

/// A single, unambiguous step in an automation sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationStep {
    pub action_type: ActionType,
    /// The specific element to act upon. Can be None for actions like Navigate.
    pub target: Option<ElementIdentifier>,
    /// The data required for the action (e.g., URL, text to type).
    pub data: Option<String>,
    /// A human-readable description of this step.
    pub description: String,
}


// ============================================================================
// Phase 3: Data for `agent-trainer` and `agent-runner`
// Represents the final, executable agent.
// ============================================================================

/// The final, runnable artifact that can perform a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub agent_type: AgentType,
    /// Path to the executable or script for this agent.
    pub executable_path: std::path::PathBuf,
}

/// The type of executable agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    /// A compiled Wasm agent.
    Wasm,
    /// A Python script.
    Python,
    /// A shell script.
    Shell,
    /// A compiled Rust agent.
    Rust,
}

/// Defines how to identify a UI element for automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementIdentifier {
    /// The method used to find the element (e.g., "css", "xpath", "id").
    pub using: String,
    /// The value of the selector.
    pub value: String,
}

/// Defines the types of actions that can be recorded and automated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    Click,
    DoubleClick,
    Type,
    Navigate,
    Scroll,
    Wait,
    Execute,
}
