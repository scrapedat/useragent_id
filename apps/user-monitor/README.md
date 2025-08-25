# App: User Monitor

## Purpose
The `user-monitor` is the starting point of the WasmAgentTrainer workflow. It is responsible for capturing all user interactions during a "recording" session and saving them in a raw, unprocessed format.

## Inputs

- **User Actions:** Real-time browser events (clicks, keystrokes, navigation, etc.) captured from the UI.

## Outputs

- **`Recording`:** A single `Recording` object that contains a chronological list of all `RecordedAction`s from the user's session. This object is saved to a persistent store (e.g., a local file or database) for the `task-learner` to consume.

_See `shared/src/types.rs` for the full data structure definitions._

Features to consider:

- User voiceover and editing of sessions
- Browser and computer user recording with a multi-faceted system mimicking human users
- High-power user monitor for task automation (computer and browser)
- Vision + LAM integration
- Gamified experience: naming agents, sharing automations to social networks
- Monetization: per-use billing, optional affiliate fees on automations

## Running with persistent Chromium and DOM recorder

1. Start the app and click "Start Chromium" to launch a persistent browser profile under `data/<app_id>/browser_profile`.
2. Click "Start DOM Recorder" to capture DOM events via CDP. This requires Node.js and the dependency below.

Install Node dependency (once):

```bash
cd useragent_id/apps/user-monitor
npm install
```

Notes:

- The recorder writes to `data/<app_id>/events/dom_session_<session_id>.jsonl`.
- OS-level input events are still recorded to `session_<session_id>.jsonl`.
