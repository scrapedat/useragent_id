use crate::UserMonitorApp;
use eframe::egui;

/// Renders the main panel of the application.
pub fn render_main_panel(app: &mut UserMonitorApp, ui: &mut egui::Ui) {
    ui.heading("WasmAgentTrainer - User Monitor");
    ui.separator();

    // --- Top Control Panel ---
    ui.horizontal(|ui| {
        if app.is_recording {
            // Show Stop button if recording
            if ui.button("Stop Recording").clicked() {
                app.stop_recording();
            }
        } else {
            // Show Start button if not recording
            if ui.button("Start Recording").clicked() {
                app.start_recording();
            }
        }
        
        ui.label(format!(
            "Status: {}",
            if app.is_recording { "Recording..." } else { "Idle" }
        ));
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Start Chromium").clicked() {
            app.start_persistent_chromium();
        }
        if ui.button("Stop Chromium").clicked() {
            app.stop_persistent_chromium();
        }
        if ui.button("Start DOM Recorder").clicked() {
            app.start_dom_recorder();
        }
        if ui.button("Stop DOM Recorder").clicked() {
            app.stop_dom_recorder();
        }
    });

    ui.horizontal(|ui| {
        let chrom_txt = if app.chromium_running() { "Chromium: running" } else { "Chromium: stopped" };
        let dom_txt = if app.dom_recorder_running() { "DOM recorder: running" } else { "DOM recorder: stopped" };
        ui.label(chrom_txt);
        ui.separator();
        ui.label(dom_txt);
    });

    ui.horizontal(|ui| {
        ui.label(format!("Node: {}", if app.node_found { "found" } else { "missing" }));
        ui.separator();
        ui.label(format!("chrome-remote-interface: {}", if app.cdp_dep_found { "installed" } else { "missing" }));
        if ui.button("Recheck deps").clicked() { app.refresh_dep_status(); }
    });
    
    ui.separator();
    
    ui.label(format!("Session ID: {}", app.session_id));
    ui.label(format!("Log Directory: {:?}", app.config.event_log_dir));

    // A real-time feed could be added here if needed, but for now,
    // we'll keep it simple as the primary output is the log file.
}
