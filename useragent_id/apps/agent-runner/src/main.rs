//! `agent-runner` Application
//!
//! This application provides a UI to select and run a trained agent.

mod executor;
mod loader;
mod ui;

use anyhow::Result;
use eframe::egui;
use shared::types::Agent;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use ui::AgentRunnerApp;
use uuid::Uuid;

/// Defines the types of requests the UI can send to the main logic thread.
pub enum RunnerRequest {
    LoadAgents,
    RunAgent(Uuid),
}

/// Defines the types of updates the main logic thread can send back to the UI.
#[derive(Debug)]
pub enum RunnerUpdate {
    AgentsLoaded(Vec<Agent>),
    AgentStarted(Uuid),
    AgentFinished(Uuid),
    StatusUpdate(String),
}

fn main() -> Result<(), eframe::Error> {
    // --- Channels for UI <-> Core Logic communication ---
    let (ui_to_core_tx, ui_to_core_rx) = mpsc::channel::<RunnerRequest>();
    let (core_to_ui_tx, core_to_ui_rx) = mpsc::channel::<RunnerUpdate>();

    // --- State for the core logic ---
    let mut agents_map: HashMap<Uuid, Agent> = HashMap::new();

    // --- Core Logic Thread ---
    let core_thread_tx = core_to_ui_tx.clone();
    let _core_thread = thread::spawn(move || {
        while let Ok(request) = ui_to_core_rx.recv() {
            match request {
                RunnerRequest::LoadAgents => {
                    let _ = core_thread_tx.send(RunnerUpdate::StatusUpdate("Loading agents...".to_string()));
                    match loader::load_all_agents() {
                        Ok(agents) => {
                            agents_map.clear();
                            for agent in &agents {
                                agents_map.insert(agent.id, agent.clone());
                            }
                            let _ = core_thread_tx.send(RunnerUpdate::AgentsLoaded(agents));
                        }
                        Err(e) => {
                            let _ = core_thread_tx.send(RunnerUpdate::StatusUpdate(format!("Error loading agents: {}", e)));
                        }
                    }
                }
                RunnerRequest::RunAgent(id) => {
                    if let Some(agent) = agents_map.get(&id) {
                        let _ = core_thread_tx.send(RunnerUpdate::AgentStarted(id));
                        let agent_clone = agent.clone();
                        let finish_tx = core_thread_tx.clone();

                        // Spawn a new thread for the potentially long-running agent execution
                        thread::spawn(move || {
                            if let Err(e) = executor::run_agent(&agent_clone) {
                                let _ = finish_tx.send(RunnerUpdate::StatusUpdate(format!("Error running agent: {}", e)));
                            }
                            let _ = finish_tx.send(RunnerUpdate::AgentFinished(id));
                        });
                    } else {
                        let _ = core_thread_tx.send(RunnerUpdate::StatusUpdate(format!("Agent with ID {} not found.", id)));
                    }
                }
            }
        }
    });

    // --- Eframe UI Setup ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Agent Runner",
        options,
        Box::new(move |cc| Box::new(AgentRunnerApp::new(cc, ui_to_core_tx, core_to_ui_rx))),
    )
}
