use eframe::egui;
use std::sync::mpsc;

use crate::{TrainingRequest, TrainingUpdate};

pub struct TrainerApp {
    /// The name of the most recently loaded task.
    loaded_task_name: Option<String>,
    /// The generated Rust code for the agent.
    generated_code: String,
    /// The ID of the last trained agent.
    trained_agent_id: Option<String>,
    /// Status message to display to the user.
    status_message: String,
    /// Channel to send requests to the main logic.
    request_tx: mpsc::Sender<TrainingRequest>,
    /// Channel to receive updates from the main logic.
    update_rx: mpsc::Receiver<TrainingUpdate>,
}

impl TrainerApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        request_tx: mpsc::Sender<TrainingRequest>,
        update_rx: mpsc::Receiver<TrainingUpdate>,
    ) -> Self {
        Self {
            loaded_task_name: None,
            generated_code: "// Generated agent code will appear here...".to_string(),
            trained_agent_id: None,
            status_message: "Ready.".to_string(),
            request_tx,
            update_rx,
        }
    }

    /// Handles updates received from the core logic thread.
    fn handle_update(&mut self, update: TrainingUpdate) {
        match update {
            TrainingUpdate::TaskLoaded(task) => {
                self.status_message = format!("Loaded task: '{}'", task.name);
                self.loaded_task_name = Some(task.name);
                self.generated_code.clear();
                self.trained_agent_id = None;
            }
            TrainingUpdate::AgentTrained(agent) => {
                self.status_message = format!("Agent {} trained successfully.", agent.id);
                self.trained_agent_id = Some(agent.id.to_string());
                // We no longer have direct access to the code here.
                // We could request it, or just show a success message.
                self.generated_code = format!("Agent code saved to {}", agent.executable_path.display());
            }
            TrainingUpdate::StatusUpdate(msg) => {
                self.status_message = msg;
            }
        }
    }
}

impl eframe::App for TrainerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for updates from the core thread on each frame
        if let Ok(update) = self.update_rx.try_recv() {
            self.handle_update(update);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Agent Trainer");
            ui.separator();

            // --- Controls ---
            ui.horizontal(|ui| {
                if ui.button("Load Latest Task").clicked() {
                    if self.request_tx.send(TrainingRequest::LoadLatestTask).is_ok() {
                        self.status_message = "Requesting to load the latest task...".to_string();
                    } else {
                        self.status_message = "Failed to send load request.".to_string();
                    }
                }

                let train_button_enabled = self.loaded_task_name.is_some();
                ui.add_enabled_ui(train_button_enabled, |ui| {
                    if ui.button("Train Agent").clicked() {
                        if self.request_tx.send(TrainingRequest::TrainAgent).is_ok() {
                            self.status_message = "Requesting to train agent...".to_string();
                        } else {
                            self.status_message = "Failed to send train request.".to_string();
                        }
                    }
                });
            });

            ui.separator();

            // --- Status Display ---
            ui.label(&self.status_message);
            if let Some(agent_id) = &self.trained_agent_id {
                ui.label(format!("Saved Agent ID: {}", agent_id));
            }

            ui.separator();

            // --- Generated Code Viewer ---
            ui.heading("Generated Agent Code");
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.generated_code.clone())
                            .font(egui::FontId::monospace(12.0))
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
        });
    }
}
