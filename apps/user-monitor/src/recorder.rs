use crossbeam_channel::{unbounded, Receiver, Sender};
use rdev::listen;
use shared::types::RecordedEvent;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

/// Listens for global input events and provides them through a channel.
pub struct InputRecorder {
    event_receiver: Option<Receiver<RecordedEvent>>,
    event_sender: Sender<RecordedEvent>,
    // A flag to signal the listening thread to stop.
    stop_signal: Arc<AtomicBool>,
}

impl InputRecorder {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            event_receiver: Some(receiver),
            event_sender: sender,
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts listening for events in a background thread.
    pub fn start_listening(&mut self) {
        let sender = self.event_sender.clone();
        self.stop_signal.store(false, Ordering::SeqCst);

        let stop_signal = self.stop_signal.clone();
        thread::spawn(move || {
            listen(move |event| {
                if !stop_signal.load(Ordering::SeqCst) {
                    let recorded_event = RecordedEvent {
                        session_id: uuid::Uuid::nil(), // Will be replaced by the app
                        timestamp: chrono::Utc::now(),
                        event_type: event.event_type.into(),
                    };
                    let _ = sender.send(recorded_event);
                }
            })
            .expect("Could not listen to events");
        });
    }

    /// Signals the listening thread to stop sending events.
    pub fn stop_listening(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);
    }

    /// Collects all events captured since the last call.
    pub fn drain_events(&self) -> Vec<RecordedEvent> {
        if let Some(receiver) = &self.event_receiver {
            receiver.try_iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Returns a sender to the event channel.
    pub fn get_event_sender(&self) -> Sender<RecordedEvent> {
        self.event_sender.clone()
    }
}

impl Default for InputRecorder {
    fn default() -> Self {
        Self::new()
    }
}
