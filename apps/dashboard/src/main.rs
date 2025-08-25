use eframe::egui;
use anyhow::Result;
use shared::core::config::AppConfig;

mod ui;
mod replay;
mod sessions;

pub struct DashboardApp {
    config: AppConfig,
    active_tab: ui::Tab,
}

impl DashboardApp {
    pub fn new(mut config: AppConfig) -> Self {
        config.app_id = "dashboard".to_string();
        Self { config, active_tab: ui::Tab::Sessions }
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("WasmAgentTrainer - Dashboard");
                ui.separator();
                ui.label(format!("Data dir: {:?}", self.config.data_dir));
            });
        });

        egui::SidePanel::left("tabs").resizable(false).show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.selectable_value(&mut self.active_tab, ui::Tab::Sessions, "Sessions");
                ui.selectable_value(&mut self.active_tab, ui::Tab::Agents, "Agents");
                ui.selectable_value(&mut self.active_tab, ui::Tab::Automations, "Automations");
                ui.selectable_value(&mut self.active_tab, ui::Tab::Issues, "Issues");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                ui::Tab::Sessions => ui::render_sessions(ui, &self.config),
                ui::Tab::Agents => ui::render_agents(ui, &self.config),
                ui::Tab::Automations => ui::render_automations(ui, &self.config),
                ui::Tab::Issues => ui::render_issues(ui, &self.config),
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();
    let config = AppConfig::load().expect("Failed to load configuration");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Dashboard",
        options,
        Box::new(|_cc| Box::new(DashboardApp::new(config))),
    )
}
