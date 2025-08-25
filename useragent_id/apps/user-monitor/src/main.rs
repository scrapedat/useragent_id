use anyhow::Result;
use eframe::egui;
use shared::core::config::AppConfig;
use shared::types::RecordedEvent;
use tracing::{error, info};
use uuid::Uuid;

mod ui;
mod recorder;
mod persistence;
mod narration;

use recorder::InputRecorder;
use persistence::EventLogger;
use narration::NarrationWatcher;
use std::process::{Child, Command, Stdio};
use std::path::PathBuf;

pub struct UserMonitorApp {
    logger: Option<EventLogger>,
    is_recording: bool,
    recorder: InputRecorder,
    session_id: Uuid,
    config: AppConfig,
    _narration_watcher: Option<NarrationWatcher>,
    chromium_child: Option<Child>,
    dom_recorder_child: Option<Child>,
    node_found: bool,
    cdp_dep_found: bool,
}

impl UserMonitorApp {
    pub fn new(mut config: AppConfig) -> Self {
        config.app_id = "user-monitor".to_string();
        Self {
            logger: None,
            is_recording: false,
            recorder: InputRecorder::new(),
            session_id: Uuid::new_v4(),
            config,
            _narration_watcher: None,
            chromium_child: None,
            dom_recorder_child: None,
            node_found: false,
            cdp_dep_found: false,
        }
    }

    pub fn start_recording(&mut self) {
        if self.is_recording {
            return;
        }
        self.is_recording = true;
        self.session_id = Uuid::new_v4();
        
        match EventLogger::new(self.session_id, &self.config.event_log_dir) {
            Ok(logger) => {
                self.logger = Some(logger);
                self.recorder.start_listening();

                // Start the narration watcher
                let live_feed_path = self.config.data_dir.join("live_feed.txt");
                let event_sender = self.recorder.get_event_sender();
                self._narration_watcher = NarrationWatcher::new(live_feed_path, event_sender, self.session_id).ok();

                info!("Started recording session: {}", self.session_id);
            }
            Err(e) => {
                error!("Failed to create event logger: {:?}", e);
                self.is_recording = false;
            }
        }
    }

    pub fn stop_recording(&mut self) {
        if !self.is_recording {
            return;
        }
        self.is_recording = false;
        self.recorder.stop_listening();
        if let Some(mut logger) = self.logger.take() {
            if let Err(e) = logger.flush() {
                error!("Failed to flush event logger: {:?}", e);
            }
        }
        info!("Stopped recording session: {}", self.session_id);
    }

    fn poll_and_log_events(&mut self) {
        if !self.is_recording {
            return;
        }

        let events = self.recorder.drain_events();
        let session_id = self.session_id; // Avoid borrowing issues

        if let Some(logger) = &mut self.logger {
            for event in events {
                // Narration events are already in the correct format.
                if let shared::types::EventType::Narration(_) = &event.event_type {
                    if let Err(e) = logger.log_event(&event) {
                        error!("Failed to log narration event: {:?}", e);
                    }
                } else {
                    if let Some(recorded_event) = to_recorded_event(event, session_id) {
                        if let Err(e) = logger.log_event(&recorded_event) {
                            error!("Failed to log event: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    pub fn start_persistent_chromium(&mut self) {
        if self.chromium_child.is_some() { return; }
        let profile_dir = self.config.browser_profile_dir.clone();
        if let Err(e) = std::fs::create_dir_all(&profile_dir) {
            error!("Failed to create browser profile dir: {:?}", e);
            return;
        }

        // Try common chromium binaries on Linux
        let candidates = ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable"];
        let bin = candidates.iter().find(|b| which::which(b).is_ok()).map(|s| s.to_string()).unwrap_or_else(|| "chromium".to_string());

        let child = Command::new(bin)
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .arg(format!("--remote-debugging-port={}", self.config.cdp_port))
            .arg("--enable-automation")
            .arg("--no-first-run")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(c) => {
                self.chromium_child = Some(c);
                info!("Started persistent Chromium at {:?}", profile_dir);
            }
            Err(e) => error!("Failed to start Chromium: {:?}", e),
        }
    }

    pub fn stop_persistent_chromium(&mut self) {
        if let Some(mut c) = self.chromium_child.take() {
            let _ = c.kill();
        }
    }

    pub fn start_dom_recorder(&mut self) {
        if self.dom_recorder_child.is_some() { return; }
        let cdp_url = format!("http://127.0.0.1:{}", self.config.cdp_port);
        let dom_dir = self.config.event_log_dir.clone();
        let out_path = dom_dir.join(format!("dom_session_{}.jsonl", self.session_id));
        let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("dom_recorder.js");

        // Spawn node script if available
        let node = which::which("node").map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "node".to_string());
        let child = Command::new(node)
            .arg(script_path)
            .env("CDP_URL", cdp_url)
            .env("OUTPUT_PATH", out_path.clone())
            .env("SESSION_ID", self.session_id.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(c) => {
                self.dom_recorder_child = Some(c);
                info!("Started DOM recorder writing to {:?}", out_path);
            }
            Err(e) => error!("Failed to start DOM recorder: {:?}", e),
        }
    }

    pub fn stop_dom_recorder(&mut self) {
        if let Some(mut c) = self.dom_recorder_child.take() {
            let _ = c.kill();
        }
    }

    pub fn refresh_dep_status(&mut self) {
        self.node_found = which::which("node").is_ok();
        let nm = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("node_modules").join("chrome-remote-interface");
        self.cdp_dep_found = nm.exists();
    }

    pub fn chromium_running(&self) -> bool {
        self.chromium_child.is_some()
    }

    pub fn dom_recorder_running(&self) -> bool {
        self.dom_recorder_child.is_some()
    }
}

/// Converts a raw `rdev::Event` into our `RecordedEvent` format.
fn to_recorded_event(event: RecordedEvent, session_id: Uuid) -> Option<RecordedEvent> {
    let event_type = match event.event_type {
        shared::types::EventType::KeyPress(key) => Some(shared::types::EventType::KeyPress(key)),
        shared::types::EventType::KeyRelease(key) => Some(shared::types::EventType::KeyRelease(key)),
        shared::types::EventType::ButtonPress(button) => Some(shared::types::EventType::ButtonPress(button)),
        shared::types::EventType::ButtonRelease(button) => Some(shared::types::EventType::ButtonRelease(button)),
        shared::types::EventType::MouseMove { x, y } => Some(shared::types::EventType::MouseMove { x, y }),
        _ => None,
    };

    event_type.map(|et| RecordedEvent {
        session_id,
        timestamp: chrono::Utc::now(),
        event_type: et,
    })
}

impl eframe::App for UserMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_and_log_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::render_main_panel(self, ui);
        });
        
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.is_recording {
            self.stop_recording();
        }
    self.stop_dom_recorder();
    self.stop_persistent_chromium();
    }
}

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load().expect("Failed to load configuration");
    
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "User Monitor",
        options,
        Box::new(|_cc| {
            let mut app = UserMonitorApp::new(config);
            app.refresh_dep_status();
            Box::new(app)
        }),
    )
}
