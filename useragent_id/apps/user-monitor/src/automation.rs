use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::monitor::ActionEvent;
use rdev::{simulate, EventType, Key};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationTask {
    pub name: String,
    pub actions: Vec<ActionStep>,
    pub triggers: Vec<AutomationTrigger>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStep {
    MouseClick { x: i32, y: i32 },
    KeyPress { key: String },
    Wait { duration_ms: u64 },
    TypeText { text: String },
    CustomScript { code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationTrigger {
    Pattern { sequence: Vec<ActionEvent> },
    Hotkey { key: String, modifiers: Vec<String> },
    Schedule { cron: String },
    VoiceCommand { phrase: String },
}

pub struct AutomationEngine {
    tasks: Vec<AutomationTask>,
}

impl AutomationEngine {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: AutomationTask) {
        self.tasks.push(task);
    }

    pub fn execute_task(&self, task_name: &str) -> Result<()> {
        if let Some(task) = self.tasks.iter().find(|t| t.name == task_name) {
            self.execute_actions(&task.actions)?;
        }
        Ok(())
    }

    fn execute_actions(&self, actions: &[ActionStep]) -> Result<()> {
        for action in actions {
            match action {
                ActionStep::MouseClick { x, y } => {
                    simulate(&EventType::MouseMove { x: *x, y: *y })?;
                    thread::sleep(Duration::from_millis(50));
                    simulate(&EventType::ButtonPress(rdev::Button::Left))?;
                    thread::sleep(Duration::from_millis(50));
                    simulate(&EventType::ButtonRelease(rdev::Button::Left))?;
                }
                ActionStep::KeyPress { key } => {
                    if let Ok(key) = parse_key(key) {
                        simulate(&EventType::KeyPress(key))?;
                        thread::sleep(Duration::from_millis(50));
                        simulate(&EventType::KeyRelease(key))?;
                    }
                }
                ActionStep::Wait { duration_ms } => {
                    thread::sleep(Duration::from_millis(*duration_ms));
                }
                ActionStep::TypeText { text } => {
                    for c in text.chars() {
                        if let Ok(key) = char_to_key(c) {
                            simulate(&EventType::KeyPress(key))?;
                            thread::sleep(Duration::from_millis(20));
                            simulate(&EventType::KeyRelease(key))?;
                        }
                    }
                }
                ActionStep::CustomScript { code } => {
                    // Execute custom WebAssembly script
                    // This would be implemented based on your WASM runtime
                }
            }
        }
        Ok(())
    }
}

fn parse_key(key: &str) -> Result<Key> {
    match key {
        "Enter" => Ok(Key::Return),
        "Tab" => Ok(Key::Tab),
        "Space" => Ok(Key::Space),
        "Backspace" => Ok(Key::Backspace),
        "Delete" => Ok(Key::Delete),
        "Escape" => Ok(Key::Escape),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            char_to_key(c)
        }
        _ => Err(anyhow::anyhow!("Invalid key: {}", key)),
    }
}

fn char_to_key(c: char) -> Result<Key> {
    match c {
        'a'..='z' | 'A'..='Z' => {
            let upper = c.to_ascii_uppercase();
            Ok(Key::KeyA + ((upper as u8) - b'A') as u32)
        }
        '0'..='9' => Ok(Key::Num0 + ((c as u8) - b'0') as u32),
        _ => Err(anyhow::anyhow!("Invalid character: {}", c)),
    }
}
