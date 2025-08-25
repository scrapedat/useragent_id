//! `agent-trainer` Application
//!
//! This application provides a UI to load a `LearnedTask` from the `task-learner`,
//! train a new `Agent` from it, and save the generated agent code to a file.

mod loader;
mod persistence;
mod trainer;
mod ui;

use anyhow::Result;
use eframe::egui;
use shared::types::{Agent, LearnedTask};
use std::sync::mpsc;
use std::thread;
use ui::TrainerApp;

/// Defines the types of requests the UI can send to the main logic thread.
pub enum TrainingRequest {
    LoadLatestTask,
    TrainAgent,
}

/// Defines the types of updates the main logic thread can send back to the UI.
#[derive(Debug)]
pub enum TrainingUpdate {
    TaskLoaded(LearnedTask),
    AgentTrained(Agent),
    StatusUpdate(String),
}

fn main() -> Result<(), eframe::Error> {
    // --- Channels for UI <-> Core Logic communication ---
    let (ui_to_core_tx, ui_to_core_rx) = mpsc::channel::<TrainingRequest>();
    let (core_to_ui_tx, core_to_ui_rx) = mpsc::channel::<TrainingUpdate>();

    // --- State for the core logic ---
    let mut loaded_task: Option<LearnedTask> = None;

    // --- Core Logic Thread ---
    let _core_thread = thread::spawn(move || {
        while let Ok(request) = ui_to_core_rx.recv() {
            match request {
                TrainingRequest::LoadLatestTask => {
                    let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate("Loading latest task...".to_string()));
                    match loader::load_latest_learned_task() {
                        Ok(task) => {
                            // Keep a copy of the loaded task for the training step
                            loaded_task = Some(task.clone());
                            let _ = core_to_ui_tx.send(TrainingUpdate::TaskLoaded(task));
                        }
                        Err(e) => {
                            let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Error loading task: {}", e)));
                        }
                    }
                }
                TrainingRequest::TrainAgent => {
                    if let Some(task) = &loaded_task {
                        let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Training agent for {}...", task.name)));
                        match trainer::train_agent_from_task(task) {
                            Ok(mut agent) => {
                                match trainer::generate_rust_code(task) {
                                    Ok(code) => {
                                        match persistence::save_agent_code(&agent.id, &code) {
                                            Ok(path) => {
                                                agent.executable_path = path;
                                                match persistence::save_agent_metadata(&agent) {
                                                    Ok(_) => {
                                                        let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Agent trained and saved.")));
                                                        let _ = core_to_ui_tx.send(TrainingUpdate::AgentTrained(agent));
                                                    }
                                                    Err(e) => {
                                                        let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Error saving agent metadata: {}", e)));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Error saving agent code: {}", e)));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Error generating agent code: {}", e)));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate(format!("Error training agent: {}", e)));
                            }
                        }
                    } else {
                        let _ = core_to_ui_tx.send(TrainingUpdate::StatusUpdate("No task loaded to train from.".to_string()));
                    }
                }
            }
        }
    });

    // --- Eframe UI Setup ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Agent Trainer",
        options,
        Box::new(move |_cc| Box::new(TrainerApp::new(_cc, ui_to_core_tx, core_to_ui_rx))),
    )
}
