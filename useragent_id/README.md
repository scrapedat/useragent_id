
# WasmAgentTrainer

## Overview
WasmAgentTrainer is a human-friendly automation platform powered by small, agentic AI modules. It enables users—even beginners—to automate tasks and build intelligent systems by teaching agents through simple examples.

## Features
- **Human-centric design:** Easy for non-experts to use and extend.
- **Agent modules:** Learn from user actions and examples.
- **Automation by example:** Create automations with simple scripts.
- **Resource-efficient:** Modular apps for monitoring, learning, training, and running agents.

## Quick Start

1. **Clone the repository**
  ```bash
  git clone https://github.com/<your-username>/<repo-name>.git
  cd WasmAgentTrainer
  ```

2. **Build and run**
  - See individual app folders for build instructions (`useragent_id/apps/`).
  - Example:  
    ```bash
    cd useragent_id/apps/agent-runner
    cargo run
    ```

3. **Try code examples**
  - Explore `code-examples/` for ready-to-use automation scripts.

## How It Works

- **User Monitor:** Captures user actions.
- **Task Learner:** Learns patterns from actions.
- **Agent Trainer:** Trains agent models.
- **Agent Runner:** Executes automations.

## For Dummies

- No AI experience required.
- Start with the examples in `code-examples/`.
- Follow comments and guides in each app folder.

## Contributing

Pull requests and suggestions are welcome! See the `CONTRIBUTING.md` for details.


## Repository hygiene (ignored artifacts)

To keep the repo lean and reproducible, common local-only and generated artifacts are ignored via `.gitignore`:

- Editor/IDE: `.vscode/`, `.snapshots/`, `.kilocode/`, `*.code-workspace`
- Rust builds: `target/`, `**/wasm32-unknown-unknown/`
- Python venv (gm_ml): `gm_ml/lib/`, `gm_ml/lib64/`, `gm_ml/include/python3.12/`, `gm_ml/dataset/bin/`, `gm_ml/pyvenv.cfg`
- Generated data: `traces/`, `trained-agents/`, `data/**/events/`, `data/**/learned_tasks/`

If you need to persist trained artifacts, export them to an external storage or release asset instead of committing them.


Below are example screenshots of the UI that human users interact with to automate their tasks:

![UI Dashboard](https://via.placeholder.com/600x300?text=UI+Dashboard)

![Automation Setup](https://via.placeholder.com/600x300?text=Automation+Setup)

![Results+and+Feedback](https://via.placeholder.com/600x300?text=Results+and+Feedback)

## License

MIT
