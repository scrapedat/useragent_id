use eframe::egui::{self, ScrollArea, plot::{Plot, Line, PlotPoints}};
use egui::{Color32, RichText, Ui, Vec2, Frame};
use egui_extras::{Size, StripBuilder, TableBuilder};
use shared::types::{DOMEvent, VoiceAnnotation};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::RwLock;
use rdev::{listen, Event as InputEvent, EventType, Key};
use std::sync::Arc;
use std::thread;
use serde::{Serialize, Deserialize};
use wasmer::{Store, Instance, Module, Value, imports};
use wasmer_wasix::{WasiEnv, Runtime};
use dashmap::DashMap;
use plotters_egui::draw_plotters;
use async_openai::{Client, types::{ChatCompletionRequestMessage, Role, CreateChatCompletionRequest}};

const NATURAL_VARIANCE_THRESHOLD: f32 = 0.15; // 15% variance in timings
const MIN_HUMAN_DELAY: u32 = 50; // Minimum delay for human-like behavior in ms
const MAX_DETECTION_SCORE: f32 = 0.8; // Threshold for high detection risk

pub struct BrowserMonitor {
    sessions: DashMap<String, BrowserSession>,
    fingerprint_pool: Vec<BrowserFingerprint>,
    risk_analyzer: RiskAnalyzer,
    pattern_matcher: PatternMatcher,
    current_session: Option<String>,
}

impl BrowserMonitor {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            fingerprint_pool: Vec::new(),
            risk_analyzer: RiskAnalyzer::new(),
            pattern_matcher: PatternMatcher::new(),
            current_session: None,
        }
    }

    pub async fn start_session(&mut self) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let fingerprint = self.generate_dynamic_fingerprint()?;
        
        let session = BrowserSession {
            id: session_id.clone(),
            start_time: Utc::now(),
            fingerprint,
            navigation_history: Vec::new(),
            detection_alerts: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
        };

        self.sessions.insert(session_id.clone(), session);
        self.current_session = Some(session_id.clone());
        Ok(session_id)
    }

    fn generate_dynamic_fingerprint(&self) -> Result<BrowserFingerprint> {
        // Implement dynamic fingerprint generation with natural variations
        // This is a key part of avoiding detection
        Ok(BrowserFingerprint {
            user_agent: self.generate_realistic_ua()?,
            webgl_vendor: self.get_random_gpu_vendor(),
            webgl_renderer: self.get_random_gpu_renderer(),
            canvas_hash: self.generate_canvas_noise()?,
            fonts: self.get_common_fonts(),
            plugins: self.generate_plugin_list(),
            screen_metrics: self.generate_realistic_metrics(),
            hardware_concurrency: self.get_random_core_count(),
            device_memory: self.get_random_memory(),
            timezone: self.get_random_timezone(),
            language_prefs: self.generate_language_list(),
        })
    }

    pub async fn record_action(&self, session_id: &str, action: NavigationAction) -> Result<()> {
        let mut session = self.sessions.get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Add natural variance to timing
        let action = self.add_timing_variance(action);
        
        // Check for detection risks
        let risks = self.risk_analyzer.analyze_action(&action, &session);
        
        if !risks.is_empty() {
            for risk in risks {
                session.detection_alerts.push(DetectionAlert {
                    timestamp: Utc::now(),
                    risk_level: risk.level,
                    detection_type: risk.detection_type,
                    context: risk.context,
                });
            }
        }

        // Record the action with natural behavior patterns
        session.navigation_history.push(NavigationEvent {
            timestamp: Utc::now(),
            action,
            metadata: self.generate_natural_metadata(),
        });

        Ok(())
    }

    fn add_timing_variance(&self, mut action: NavigationAction) -> NavigationAction {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Add natural variance to timing
        if let Some(timing) = &mut action.timing {
            let variance = rng.gen_range(-NATURAL_VARIANCE_THRESHOLD..NATURAL_VARIANCE_THRESHOLD);
            timing.duration = ((timing.duration as f32) * (1.0 + variance)) as u32;
            timing.start_delay = ((timing.start_delay as f32) * (1.0 + variance)) as u32;
        }

        action
    }

// VM Types for system-wide recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMState {
    pub memory: Vec<u8>,
    pub registers: Vec<u64>,
    pub instruction_ptr: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMSnapshot {
    pub state: VMState,
    pub timestamp: DateTime<Utc>,
}

// Analysis Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAnalysis {
    pub event: ActionEvent,
    pub ai_analysis: String,
    pub confidence: f32,
    pub suggested_optimizations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSession {
    pub id: String,
    pub start_time: DateTime<Utc>,
    pub fingerprint: BrowserFingerprint,
    pub navigation_history: Vec<NavigationEvent>,
    pub detection_alerts: Vec<DetectionAlert>,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionAlert {
    pub timestamp: DateTime<Utc>,
    pub risk_level: RiskLevel,
    pub detection_type: DetectionType,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionType {
    WebDriver,
    Automation,
    ConsistentTiming,
    UnusualPatterns,
    FingerprintMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub response_times: Vec<(DateTime<Utc>, u32)>,
    pub memory_usage: Vec<(DateTime<Utc>, u64)>,
    pub cpu_usage: Vec<(DateTime<Utc>, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalytics {
    pub total_actions: usize,
    pub action_frequency: Vec<(DateTime<Utc>, usize)>,
    pub common_patterns: Vec<(Vec<ActionEvent>, usize)>,
    pub risk_assessment: RiskAssessment,
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
}
    pub efficiency_score: f32,
}

struct ActionEvent {
    timestamp: DateTime<Utc>,
    event: DOMEvent,
    voice: Option<VoiceAnnotation>,
    mouse_pos: (i32, i32),
    window_title: Option<String>,
}

pub enum RecordingMode {
    Idle,
    Recording,
    Paused,
}

#[derive(Default)]
pub enum ViewMode {
    #[default]
    Timeline,
    Analysis,
    Settings,
}

pub struct SessionState {
    pub events: Vec<DOMEvent>,
    pub voice_annotations: Vec<VoiceAnnotation>,
    pub start_time: Option<Instant>,
    pub elapsed: Duration,
    pub mode: RecordingMode,
}

pub struct MonitorApp {
    session: Arc<RwLock<SessionState>>,
    view_mode: ViewMode,
    voice_enabled: bool,
    status_message: String,
    event_sender: Sender<DOMEvent>,
    event_receiver: Receiver<DOMEvent>,
    voice_receiver: Receiver<String>,
    mouse_tracker: Mouse,
    save_path: PathBuf,
    window_pos: Option<Vec2>,
    show_settings: bool,
    auto_save: bool,
    auto_save_interval: Duration,
    last_save: Instant,
    
    // VM Integration
    vm_store: Store,
    vm_instance: Option<Instance>,
    vm_snapshots: Vec<VMSnapshot>,
    vm_state: Arc<RwLock<VMState>>,
    
    // Analysis
    ai_client: Client,
    action_analysis: DashMap<DateTime<Utc>, ActionAnalysis>,
    session_analytics: Arc<RwLock<SessionAnalytics>>,
    analysis_cache: DashMap<String, String>,
    
    // AI Agent Integration
    ai_agent: Option<AIAgent>,
    agent_tasks: Arc<RwLock<Vec<AgentTask>>>,
    automation_suggestions: Vec<String>,
    
    // Visualization
    timeline_plot: Plot,
    heatmap_data: Vec<(f64, f64, f64)>,
    pattern_graph: Vec<(Vec<ActionEvent>, Vec<(f64, f64)>)>,
    
    // Persistence
    session_history: Vec<SessionState>,
    replay_speed: f32,
    replay_index: usize,
    is_replaying: bool,
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up channels for event communication
        let (event_sender, event_receiver) = unbounded();
        let (_, voice_receiver) = unbounded();

        // Set up save directory
        let proj_dirs = ProjectDirs::from("com", "useragent", "monitor")
            .expect("Failed to get project directories");
        let save_path = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&save_path).expect("Failed to create save directory");

        // Initialize VM environment
        let store = Store::default();
        let vm_state = Arc::new(RwLock::new(VMState {
            memory: Vec::new(),
            registers: vec![0; 16],
            instruction_ptr: 0,
        }));

        // Initialize AI client
        let ai_client = Client::new();
        
        // Initialize analytics
        let session_analytics = Arc::new(RwLock::new(SessionAnalytics {
            total_actions: 0,
            action_frequency: Vec::new(),
            common_patterns: Vec::new(),
            efficiency_score: 0.0,
        }));

        // Initialize mouse tracker
        let mouse_tracker = Mouse::new();

        // Initialize session state
        let session = Arc::new(RwLock::new(SessionState {
            events: Vec::new(),
            voice_annotations: Vec::new(),
            start_time: None,
            elapsed: Duration::default(),
            mode: RecordingMode::Idle,
        }));

        // Set up global event listener
        let event_sender_clone = event_sender.clone();
        let session_clone = session.clone();
        
        std::thread::spawn(move || {
            if let Err(error) = listen(move |event| {
                if let Ok(mut session) = session_clone.write() {
                    if matches!(session.mode, RecordingMode::Recording) {
                        match event.event_type {
                            EventType::KeyPress(Key::Escape) => {
                                session.mode = RecordingMode::Paused;
                            }
                            EventType::KeyPress(_) | 
                            EventType::ButtonPress(_) |
                            EventType::MouseMove { .. } => {
                                let _ = event_sender_clone.try_send(DOMEvent {
                                    event_type: format!("{:?}", event.event_type),
                                    element_tag: String::new(), // Will be populated by analyzer
                                    xpath: String::new(),       // Will be populated by analyzer
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }) {
                eprintln!("Error: {:?}", error);
            }
        });

        Self {
            session,
            view_mode: ViewMode::default(),
            voice_enabled: false,
            status_message: "Ready to record".to_string(),
            event_sender,
            event_receiver,
            voice_receiver,
            mouse_tracker,
            save_path,
            window_pos: None,
            show_settings: false,
            auto_save: true,
            auto_save_interval: Duration::from_secs(300), // 5 minutes
            last_save: Instant::now(),
        }
    }

    fn start_recording(&mut self) {
        let mut session = self.session.write();
        session.mode = RecordingMode::Recording;
        session.start_time = Some(Instant::now());
        self.status_message = "Recording started...".to_string();
    }

    fn pause_recording(&mut self) {
        let mut session = self.session.write();
        session.mode = RecordingMode::Paused;
        if let Some(start) = session.start_time {
            session.elapsed += start.elapsed();
        }
        self.status_message = "Recording paused".to_string();
    }

    fn resume_recording(&mut self) {
        let mut session = self.session.write();
        session.mode = RecordingMode::Recording;
        session.start_time = Some(Instant::now());
        self.status_message = "Recording resumed...".to_string();
    }

    fn clear_session(&mut self) {
        let mut session = self.session.write();
        session.events.clear();
        session.voice_annotations.clear();
        session.start_time = None;
        session.elapsed = Duration::default();
        session.mode = RecordingMode::Idle;
        self.status_message = "Session cleared".to_string();
    }

    fn toggle_voice(&mut self) {
        self.voice_enabled = !self.voice_enabled;
        self.status_message = if self.voice_enabled {
            "Voice recording enabled".to_string()
        } else {
            "Voice recording disabled".to_string()
        };
    }

    fn save_session(&mut self) -> Result<()> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("session_{}.json", timestamp);
        let save_path = self.save_path.join(filename);

        let session = self.session.read();
        let session_data = serde_json::json!({
            "events": session.events,
            "voice_annotations": session.voice_annotations,
            "timestamp": timestamp.to_string(),
            "duration": session.elapsed.as_secs(),
        });

        fs::write(save_path, serde_json::to_string_pretty(&session_data)?)?;
        self.last_save = Instant::now();
        Ok(())
    }

    fn check_auto_save(&mut self) {
        if self.auto_save && self.last_save.elapsed() >= self.auto_save_interval {
            if let Err(e) = self.save_session() {
                self.status_message = format!("Auto-save failed: {}", e);
            } else {
                self.status_message = "Session auto-saved".to_string();
            }
        }
    }

    fn render_timeline(&mut self, ui: &mut Ui) {
        let frame = Frame::dark_canvas(ui.style());
        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading(RichText::new("Action Timeline").size(24.0));
                ui.add_space(8.0);

                // Timeline controls
                ui.horizontal(|ui| {
                    let session = self.session.read();
                    match session.mode {
                        RecordingMode::Recording => {
                            if ui.button(RichText::new("⏸ Pause").color(Color32::YELLOW)).clicked() {
                                drop(session);
                                self.pause_recording();
                            }
                        }
                        RecordingMode::Paused => {
                            if ui.button(RichText::new("▶ Resume").color(Color32::GREEN)).clicked() {
                                drop(session);
                                self.resume_recording();
                            }
                        }
                        RecordingMode::Idle => {
                            if ui.button(RichText::new("⏺ Start Recording").color(Color32::GREEN)).clicked() {
                                drop(session);
                                self.start_recording();
                            }
                        }
                    }

                    if ui.button(RichText::new("💾 Save").color(Color32::LIGHT_BLUE)).clicked() {
                        if let Err(e) = self.save_session() {
                            self.status_message = format!("Error saving: {}", e);
                        } else {
                            self.status_message = "Session saved successfully".to_string();
                        }
                    }

                    if ui.button(RichText::new("🧹 Clear").color(Color32::LIGHT_RED)).clicked() {
                        self.clear_session();
                    }

                    let voice_text = if self.voice_enabled { "🎤 Voice: ON" } else { "🎤 Voice: OFF" };
                    if ui.button(RichText::new(voice_text)
                        .color(if self.voice_enabled { Color32::GREEN } else { Color32::GRAY }))
                        .clicked() {
                        self.toggle_voice();
                    }
                });
                
                ui.add_space(8.0);

                // Session stats
                ui.horizontal(|ui| {
                    let session = self.session.read();
                    if let Some(start_time) = session.start_time {
                        let elapsed = start_time.elapsed();
                        ui.label(RichText::new(format!(
                            "Recording time: {:02}:{:02}:{:02}",
                            elapsed.as_secs() / 3600,
                            (elapsed.as_secs() % 3600) / 60,
                            elapsed.as_secs() % 60
                        )).color(Color32::LIGHT_YELLOW));
                    }
                    ui.label(RichText::new(format!(
                        "Events: {} | Voice Notes: {}",
                        session.events.len(),
                        session.voice_annotations.len()
                    )).color(Color32::LIGHT_GRAY));
                });

                ui.add_space(8.0);

                // Timeline view
                ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        let session = self.session.read();
                        StripBuilder::new(ui)
                            .size(Size::remainder())
                            .vertical(|mut strip| {
                                for (i, event) in session.events.iter().enumerate() {
                                    strip.cell(|ui| {
                                        ui.horizontal(|ui| {
                                            // Timestamp
                                            ui.label(RichText::new(format!("{:03} ", i))
                                                .color(Color32::LIGHT_BLUE)
                                                .monospace());

                                            // Event type icon
                                            let icon = match event.event_type.as_str() {
                                                "click" => "🖱️",
                                                "keypress" => "⌨️",
                                                "mousemove" => "➡️",
                                                _ => "📍",
                                            };
                                            ui.label(icon);

                                            // Event details
                                            ui.label(format!("{} on {}", event.event_type, event.element_tag));

                                            // Voice annotation if any
                                            if let Some(voice) = session.voice_annotations.get(i) {
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.label(RichText::new(&voice.text)
                                                        .italics()
                                                        .color(Color32::LIGHT_GREEN));
                                                    ui.label("🗣️");
                                                });
                                            }
                                        });
                                    });
                                }
                            });
                    });
            });
        });
    }

    fn render_controls(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Recording Controls");
            ui.add_space(8.0);

            let record_text = if self.is_recording { "Stop Recording" } else { "Start Recording" };
            let record_color = if self.is_recording { Color32::RED } else { Color32::GREEN };
            
            if ui.button(RichText::new(record_text).color(record_color)).clicked() {
                self.is_recording = !self.is_recording;
                if self.is_recording {
                    self.recording_started = Some(Instant::now());
                    self.status_message = "Recording started...".to_string();
                } else {
                    if let Err(e) = self.save_session() {
                        self.status_message = format!("Error saving session: {}", e);
                    } else {
                        self.status_message = "Session saved successfully".to_string();
                    }
                }
            }

            // Voice recording toggle
            let voice_text = if self.voice_enabled { "Voice Input: ON" } else { "Voice Input: OFF" };
            if ui.button(RichText::new(voice_text).color(if self.voice_enabled { Color32::GREEN } else { Color32::GRAY })).clicked() {
                self.voice_enabled = !self.voice_enabled;
            }

            // Clear button
            if ui.button(RichText::new("Clear").color(Color32::YELLOW)).clicked() {
                self.events.clear();
                self.voice_annotations.clear();
                self.status_message = "Session cleared".to_string();
            }

            // Status message
            ui.add_space(8.0);
            ui.label(RichText::new(&self.status_message).color(Color32::LIGHT_YELLOW));

            // Recording duration
            if let Some(started) = self.recording_started {
                if self.is_recording {
                    let duration = started.elapsed();
                    ui.label(format!(
                        "Recording time: {:02}:{:02}",
                        duration.as_secs() / 60,
                        duration.as_secs() % 60
                    ));
                }
            }
        });
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for new events
        while let Ok(event) = self.event_receiver.try_recv() {
            if let Ok(mut session) = self.session.write() {
                session.events.push(event);
            }
        }

        // Check for voice input
        if self.voice_enabled {
            if let Ok(text) = self.voice_receiver.try_recv() {
                if let Ok(mut session) = self.session.write() {
                    session.voice_annotations.push(VoiceAnnotation {
                        text,
                        confidence: 1.0,
                    });
                }
            }
        }

        // Auto-save check
        self.check_auto_save();

        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Top bar
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("User Action Monitor").size(32.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚙️ Settings").clicked() {
                            self.show_settings = !self.show_settings;
                        }
                    });
                });

                ui.separator();

                // Main content
                match self.view_mode {
                    ViewMode::Timeline => self.render_timeline(ui),
                    ViewMode::Analysis => self.render_analysis(ui),
                    ViewMode::Settings => self.render_settings(ui),
                }

                // Status bar
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&self.status_message)
                            .color(Color32::LIGHT_YELLOW));
                    });
                });
            });
        });

        // Settings window
        if self.show_settings {
            Window::new("Settings")
                .fixed_size(Vec2::new(300.0, 200.0))
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.auto_save, "Auto-save sessions");
                    if self.auto_save {
                        ui.horizontal(|ui| {
                            ui.label("Auto-save interval:");
                            ui.add(egui::DragValue::new(&mut self.auto_save_interval)
                                .speed(1.0)
                                .suffix("s"));
                        });
                    }
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_settings = false;
                    }
                });
        }

        // Request repaint for real-time updates
        if matches!(self.session.read().mode, RecordingMode::Recording) {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
