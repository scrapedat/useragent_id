use eframe::egui;
use shared::types::Agent;
use std::sync::mpsc;
use uuid::Uuid;

use crate::{RunnerRequest, RunnerUpdate};

pub struct AgentRunnerApp {
    /// List of all available agents found in the `trained-agents` directory.
    available_agents: Vec<Agent>,
    /// The ID of the agent currently selected by the user.
    selected_agent_id: Option<Uuid>,
    /// The ID of the agent currently running.
    running_agent_id: Option<Uuid>,
    /// Status message to display to the user.
    status_message: String,
    /// Channel to send requests to the main logic.
    request_tx: mpsc::Sender<RunnerRequest>,
    /// Channel to receive updates from the main logic.
    update_rx: mpsc::Receiver<RunnerUpdate>,
}

impl AgentRunnerApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        request_tx: mpsc::Sender<RunnerRequest>,
        update_rx: mpsc::Receiver<RunnerUpdate>,
    ) -> Self {
        // Request an initial load of agents when the app starts
        request_tx.send(RunnerRequest::LoadAgents).ok();

        Self {
            available_agents: Vec::new(),
            selected_agent_id: None,
            running_agent_id: None,
            status_message: "Loading agents...".to_string(),
            request_tx,
            update_rx,
        }
    }

    /// Handles updates received from the core logic thread.
    fn handle_update(&mut self, update: RunnerUpdate) {
        match update {
            RunnerUpdate::AgentsLoaded(agents) => {
                self.status_message = format!("Found {} agents.", agents.len());
                self.available_agents = agents;
            }
            RunnerUpdate::AgentStarted(id) => {
                self.running_agent_id = Some(id);
                self.status_message = format!("Agent {} is running...", id);
            }
            RunnerUpdate::AgentFinished(id) => {
                if self.running_agent_id == Some(id) {
                    self.running_agent_id = None;
                    self.status_message = format!("Agent {} finished.", id);
                }
            }
            RunnerUpdate::StatusUpdate(msg) => {
                self.status_message = msg;
            }
        }
    }
}

impl eframe::App for AgentRunnerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for updates from the core thread on each frame
        if let Ok(update) = self.update_rx.try_recv() {
            self.handle_update(update);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Agent Runner");
            ui.separator();

            // --- Controls ---
            ui.horizontal(|ui| {
                if ui.button("Refresh Agent List").clicked() {
                    self.request_tx.send(RunnerRequest::LoadAgents).ok();
                }

                let run_button_enabled = self.selected_agent_id.is_some() && self.running_agent_id.is_none();
                ui.add_enabled_ui(run_button_enabled, |ui| {
                    if ui.button("Run Selected Agent").clicked() {
                        if let Some(id) = self.selected_agent_id {
                            self.request_tx.send(RunnerRequest::RunAgent(id)).ok();
                        }
                    }
                });
            });

            ui.separator();

            // --- Status Display ---
            ui.label(&self.status_message);

            ui.separator();

            // --- Agent List ---
            ui.heading("Available Agents");
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                for agent in &self.available_agents {
                    let is_selected = self.selected_agent_id == Some(agent.id);
                    if ui.selectable_label(is_selected, &agent.name).clicked() {
                        self.selected_agent_id = Some(agent.id);
                    }
                }
            });
        });
    }
}
