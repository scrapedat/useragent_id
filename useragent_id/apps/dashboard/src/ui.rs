use eframe::egui;
use shared::core::config::AppConfig;
use crate::sessions;
use crate::replay::align_narration_to_dom;
use crate::sessions::SessionSummary;
use std::path::PathBuf;
use std::io::BufRead;
use std::process::Command;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Sessions,
    Agents,
    Automations,
    Issues,
}

pub fn render_sessions(ui: &mut egui::Ui, config: &AppConfig) {
    ui.heading("Sessions");
    ui.separator();

    // Replay control state (shared across sessions)
    let mut speed_ms: i32 = ui
        .data(|d| d.get_temp("replay_speed_ms".into()))
        .unwrap_or(250);
    let pause_file_path = std::env::temp_dir().join("wam_replay_pause.toggle");
    let step_file_path = std::env::temp_dir().join("wam_replay_step.signal");
    let stop_file_path = std::env::temp_dir().join("wam_replay_stop.signal");
    let progress_file_path = std::env::temp_dir().join("wam_replay_progress.json");
    let is_paused_now = pause_file_path.exists();

    let mut sessions_cache: Vec<SessionSummary> = ui
        .data(|d| d.get_temp("sessions_list".into()))
        .unwrap_or_default();

    if sessions_cache.is_empty() {
        if let Ok(list) = sessions::load_sessions(config) {
            sessions_cache = list;
            ui.data_mut(|d| d.insert_temp("sessions_list".into(), sessions_cache.clone()));
        }
    }

    if ui.button("Refresh").clicked() {
        if let Ok(list) = sessions::load_sessions(config) {
            sessions_cache = list;
            ui.data_mut(|d| d.insert_temp("sessions_list".into(), sessions_cache.clone()));
        }
    }

    ui.add_space(4.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (idx, s) in sessions_cache.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} events", s.num_events));
                    if let (Some(st), Some(en)) = (s.start, s.end) {
                        ui.label(format!("{} - {}", st.format("%H:%M:%S"), en.format("%H:%M:%S")));
                    }
                    ui.label(format!("Narrations: {}", s.num_narrations));
                    ui.label(format!("DOM: {}", s.num_dom_events));
                });

                let mut title = s.title.clone();
                let mut desc = s.description.clone();
                ui.label(format!("Session: {}", s.session_id));
                ui.text_edit_singleline(&mut title);
                ui.text_edit_multiline(&mut desc);

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        s.title = title.clone();
                        s.description = desc.clone();
                        let _ = sessions::save_meta_for(s);
                        // Update cache
                        ui.data_mut(|d| {
                            let mut current: Vec<SessionSummary> = d.get_temp("sessions_list".into()).unwrap_or_default();
                            if let Some(slot) = current.get_mut(idx) {
                                *slot = s.clone();
                            }
                            d.insert_temp("sessions_list".into(), current);
                        });
                    }
                    ui.label(format!("Log: {:?}", s.log_path.file_name().unwrap_or_default()));
                    // Speed slider (50ms - 1200ms)
                    ui.add(
                        egui::Slider::new(&mut speed_ms, 50..=1200)
                            .text("Speed (ms)")
                            .clamp_to_range(true),
                    );
                    // Persist speed control immediately
                    ui.data_mut(|d| d.insert_temp("replay_speed_ms".into(), speed_ms));

                    // Pause/Resume toggle using a temp file that the replayer watches
                    if is_paused_now {
                        if ui.button("Resume").clicked() {
                            let _ = std::fs::remove_file(&pause_file_path);
                        }
                    } else {
                        if ui.button("Pause").clicked() {
                            // Create (or touch) the pause file
                            let _ = std::fs::write(&pause_file_path, b"pause");
                        }
                    }
                    // Step-by-step mode: toggle by creating/removing a remembered flag
                    let mut step_mode: bool = ui.data(|d| d.get_temp("replay_step_mode".into())).unwrap_or(false);
                    if ui.selectable_label(step_mode, "Step Mode").clicked() {
                        step_mode = !step_mode;
                        ui.data_mut(|d| d.insert_temp("replay_step_mode".into(), step_mode));
                    }
                    if step_mode {
                        if ui.button("Step").clicked() {
                            // touch the step signal file so replayer advances one step
                            let _ = std::fs::write(&step_file_path, b"step");
                        }
                    }
                    // Stop button: signal the replayer to end early
                    if ui.button("Stop").clicked() {
                        let _ = std::fs::write(&stop_file_path, b"stop");
                    }
                    if ui.button("Replay DOM").clicked() {
                        let dom_log = s
                            .log_path
                            .parent()
                            .map(|p| p.join(format!("dom_session_{}.jsonl", s.session_id)))
                            .unwrap_or(PathBuf::new());
                        if dom_log.exists() {
                            // Prepare captions by aligning narration to DOM lines
                            let os_lines: Vec<shared::types::RecordedEvent> = {
                                let f = std::fs::File::open(&s.log_path);
                                if let Ok(f) = f {
                                    let rdr = std::io::BufReader::new(f);
                                    rdr.lines().filter_map(|l| l.ok()).filter_map(|l| {
                                        let parsed: Result<shared::types::RecordedEvent, _> = serde_json::from_str(&l);
                                        parsed.ok()
                                    }).collect()
                                } else { Vec::new() }
                            };
                            let dom_lines_raw: Vec<String> = {
                                let f = std::fs::File::open(&dom_log);
                                if let Ok(f) = f {
                                    let rdr = std::io::BufReader::new(f);
                                    rdr.lines().filter_map(|l| l.ok()).collect()
                                } else { Vec::new() }
                            };
                            let captions_pairs = align_narration_to_dom(&dom_lines_raw, &os_lines);
                            let captions_json = serde_json::to_string(&captions_pairs.iter().map(|(i, t)| serde_json::json!({"index": i, "text": t})).collect::<Vec<_>>()).unwrap_or("[]".to_string());
                            // Spawn node replayer (expects Chromium already running via user-monitor)
                            // Clear progress/stop/step markers
                            let _ = std::fs::remove_file(&progress_file_path);
                            let _ = std::fs::remove_file(&stop_file_path);
                            if !ui.data(|d| d.get_temp::<bool>("replay_step_mode".into()).unwrap_or(false)) {
                                let _ = std::fs::remove_file(&step_file_path);
                            }
                            // Resolve script path robustly by searching upwards for the repo root
                            let replayer_rel = Path::new("apps/user-monitor/src/dom_replayer.js");
                            let script_path = {
                                // try current_dir and up to 5 ancestors
                                let mut found: Option<PathBuf> = None;
                                if let Ok(mut cur) = std::env::current_dir() {
                                    for _ in 0..6 {
                                        let candidate = cur.join(replayer_rel);
                                        if candidate.exists() { found = Some(candidate); break; }
                                        if !cur.pop() { break; }
                                    }
                                }
                                found.unwrap_or_else(|| replayer_rel.to_path_buf())
                            };
                            let _ = Command::new("node")
                                .env("CDP_URL", "http://127.0.0.1:9222")
                                .env("INPUT_PATH", dom_log)
                                .env("SPEED_MS", speed_ms.to_string())
                                .env("CAPTIONS", captions_json)
                                .env("PAUSE_FILE", pause_file_path.as_os_str())
                                .env("PROGRESS_FILE", progress_file_path.as_os_str())
                                .env("STOP_FILE", stop_file_path.as_os_str())
                                .env("STEP_FILE", if ui.data(|d| d.get_temp::<bool>("replay_step_mode".into()).unwrap_or(false)) { step_file_path.as_os_str() } else { std::ffi::OsStr::new("") })
                                .arg(&script_path)
                                .spawn();
                        }
                    }
                });

                // Quickview panels
                ui.collapsing("Preview logs / Chat", |ui| {
                    // Progress indicator (reads a small JSON written by replayer)
                    if progress_file_path.exists() {
                        if let Ok(txt) = std::fs::read_to_string(&progress_file_path) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                                if let (Some(i), Some(t)) = (v.get("index").and_then(|x| x.as_u64()), v.get("total").and_then(|x| x.as_u64())) {
                                    let pct = if t > 0 { (i.min(t) as f32 / t as f32) * 100.0 } else { 0.0 };
                                    ui.label(format!("Replay progress: {}/{} ({:.0}%)", i, t, pct));
                                }
                            }
                        }
                    }
                    let os_log = s.log_path.clone();
                    let dom_log = s
                        .log_path
                        .parent()
                        .map(|p| p.join(format!("dom_session_{}.jsonl", s.session_id)))
                        .unwrap_or(PathBuf::new());
                    if dom_log.exists() {
                        if let Ok(Some((url, title))) = sessions::last_dom_location(&dom_log) {
                            ui.horizontal(|ui| {
                                ui.label("Last location:");
                                ui.label(egui::RichText::new(url).monospace());
                                if let Some(t) = title { ui.label(format!("— {}", t)); }
                            });
                        }
                    }
                    ui.columns(3, |cols| {
                        cols[0].label("OS Events (tail)");
                        if let Ok(lines) = sessions::tail_lines(&os_log, 20) {
                            for l in lines { cols[0].label(egui::RichText::new(l).monospace()); }
                        }
                        cols[1].label("DOM Events (tail)");
                        if dom_log.exists() {
                            if let Ok(lines) = sessions::tail_lines(&dom_log, 20) {
                                for l in lines { cols[1].label(egui::RichText::new(l).monospace()); }
                            }
                        } else {
                            cols[1].label("No DOM log");
                        }
                        cols[2].label("Narration / Chat");
                        // Narration tail
                        if let Ok(lines) = sessions::tail_narrations(&os_log, 10) {
                            for l in lines { cols[2].label(format!("👤 {}", l)); }
                        }
                        // Simple input for AI chat stub (not persisted yet)
                        cols[2].separator();
                        let mut prompt: String = cols[2].data(|d| d.get_temp("chat_prompt".into())).unwrap_or_default();
                        if cols[2].text_edit_singleline(&mut prompt).lost_focus() && cols[2].input(|i| i.key_pressed(egui::Key::Enter)) {
                            // For now, echo; later wire to a local model or service
                            cols[2].data_mut(|d| d.insert_temp("chat_last".into(), format!("🤖 echo: {}", prompt)));
                            prompt.clear();
                        }
                        cols[2].data_mut(|d| d.insert_temp("chat_prompt".into(), prompt.clone()));
                        let last: Option<String> = cols[2].data(|d| d.get_temp("chat_last".into()));
                        if let Some(resp) = last {
                            cols[2].label(resp);
                        }
                    });
                });

                // New: Traces panel
                ui.collapsing("Execution Traces", |ui| {
                    if s.traces.is_empty() {
                        ui.label("No traces found.");
                    } else {
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            for tr in &s.traces {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(tr.file_name().unwrap_or_default().to_string_lossy()).monospace());
                                    if ui.button("Open").clicked() {
                                        // Store selected trace path in UI memory to render details below
                                        ui.data_mut(|d| d.insert_temp("selected_trace".into(), tr.to_string_lossy().to_string()));
                                    }
                                });
                            }
                        });
                        let selected_trace: Option<String> = ui.data(|d| d.get_temp("selected_trace".into()));
                        if let Some(sel) = selected_trace {
                            if let Ok(txt) = std::fs::read_to_string(&sel) {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                                    let agent = val.get("agent_type").and_then(|v| v.as_str()).unwrap_or("");
                                    let status = val.get("status").map(|v| v.to_string()).unwrap_or_default();
                                    let when = val.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
                                    ui.label(format!("Agent: {} — Status: {} — At: {}", agent, status, when));
                                    let mut raw = val.to_string();
                                    ui.add(egui::TextEdit::multiline(&mut raw).desired_rows(6));
                                } else {
                                    ui.label("Failed to parse trace JSON");
                                }
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }
    });
}

pub fn render_agents(ui: &mut egui::Ui, _config: &AppConfig) {
    ui.heading("Agents");
    ui.separator();
    ui.label("TODO: Show agents gallery with name/description/image and basic actions.");
}

pub fn render_automations(ui: &mut egui::Ui, _config: &AppConfig) {
    ui.heading("Automations");
    ui.separator();
    ui.label("TODO: Create schedules/timers and notifications to LAM/agents.");
}

pub fn render_issues(ui: &mut egui::Ui, _config: &AppConfig) {
    ui.heading("Issues");
    ui.separator();
    ui.label("TODO: Report problems with automations; persist and display history.");
}
