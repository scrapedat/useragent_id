use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use std::sync::mpsc::Sender;
use anyhow::Result;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};
use std::sync::Arc;
use parking_lot::Mutex;
use hound::{WavSpec, WavWriter};
use std::io::BufWriter;
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 16000;
const SILENCE_THRESHOLD: f32 = 0.1;
const SILENCE_DURATION: Duration = Duration::from_secs(1);

pub struct VoiceRecorder {
    tx: Sender<String>,
    whisper: Arc<WhisperContext>,
    buffer: Arc<Mutex<Vec<f32>>>,
    last_sound: Arc<Mutex<Option<Instant>>>,
    temp_dir: PathBuf,
}

impl VoiceRecorder {
    pub fn new(tx: Sender<String>) -> Result<Self> {
        // Load whisper model
        let whisper = WhisperContext::new("models/ggml-base.en.bin")?;
        
        // Create temp directory for WAV files
        let temp_dir = std::env::temp_dir().join("user_monitor_voice");
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            tx,
            whisper: Arc::new(whisper),
            buffer: Arc::new(Mutex::new(Vec::new())),
            last_sound: Arc::new(Mutex::new(None)),
            temp_dir,
        })
    }

    pub fn start_recording(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = self.buffer.clone();
        let last_sound = self.last_sound.clone();
        let whisper = self.whisper.clone();
        let tx = self.tx.clone();
        let temp_dir = self.temp_dir.clone();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = buffer.lock();
                let mut last_sound = last_sound.lock();
                
                // Check if there's sound in the current buffer
                let has_sound = data.iter().any(|&s| s.abs() > SILENCE_THRESHOLD);
                
                if has_sound {
                    *last_sound = Some(Instant::now());
                    buffer.extend_from_slice(data);
                } else if let Some(last) = *last_sound {
                    if last.elapsed() >= SILENCE_DURATION && !buffer.is_empty() {
                        // Save buffer to WAV file
                        if let Ok(path) = save_wav(&buffer, &temp_dir) {
                            // Process audio with whisper
                            if let Ok(text) = process_audio(&whisper, &path) {
                                if !text.trim().is_empty() {
                                    let _ = tx.send(text);
                                }
                            }
                        }
                        buffer.clear();
                        *last_sound = None;
                    }
                }
            },
            move |err| eprintln!("Error in audio stream: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(())
    }
}

fn save_wav(samples: &[f32], temp_dir: &PathBuf) -> Result<PathBuf> {
    let file_path = temp_dir.join(format!("recording_{}.wav", Instant::now().elapsed().as_millis()));
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::new(
        BufWriter::new(File::create(&file_path)?),
        spec,
    )?;

    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    
    Ok(file_path)
}

fn process_audio(whisper: &WhisperContext, path: &PathBuf) -> Result<String> {
    // Read WAV file
    let mut reader = hound::WavReader::open(path)?;
    let samples: Vec<f32> = reader.samples::<f32>()
        .filter_map(Result::ok)
        .collect();

    // Set up whisper parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false)
        .set_print_progress(false)
        .set_print_realtime(false)
        .set_print_timestamps(false);

    // Process audio
    let mut state = whisper.create_state()?;
    state.full(params, &samples[..])?;

    // Extract text
    let num_segments = state.full_n_segments()?;
    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
            text.push_str(&segment);
            text.push(' ');
        }
    }

    // Clean up temp file
    std::fs::remove_file(path)?;

    Ok(text)
}
