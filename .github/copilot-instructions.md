# WasmAgentTrainer Project Guide for AI Agents

This document provides essential guidance for working on the WasmAgentTrainer codebase.

## 1. High-Level Architecture

The project is a hybrid system combining a Rust application suite with a Python-based Machine Learning environment.

- **Primary Application (`useragent_id/`):** A Rust Cargo workspace containing several modular applications. The core idea is to monitor user actions, learn tasks, and train small, efficient agents to perform automation. The data flows between components: `user-monitor` captures raw events, `task-learner` analyzes them to define tasks, `agent-trainer` produces a model, and `agent-runner` executes it.
- **ML Fine-Tuning (`useragent_id/gm_ml/`):** A self-contained Python environment for fine-tuning a Small Language Model (SLM) on the project's own Rust source code. This is a meta-task to create a specialized coding assistant.

## 2. Rust Development Workflow (`useragent_id/`)

The Rust part of the project is a standard Cargo workspace.

- **Workspace Structure:** The main crates are located in `useragent_id/apps/`:
    - `user-monitor`: Captures user inputs and system events.
    - `task-learner`: Analyzes event data to define automatable tasks.
    - `agent-trainer`: Trains the agent models.
    - `agent-runner`: Executes trained agents.
    - `shared`: A library for common data types and functions used across the other crates.

- **Building & Running:**
    - To run a specific app, navigate to its directory and use `cargo run`. For example:
      ```bash
      cd useragent_id/apps/user-monitor
      cargo run
      ```
    - To build the entire workspace, run `cargo build --workspace` from the `useragent_id/` directory.

- **Dependencies:** Workspace-wide dependencies are managed in `useragent_id/Cargo.toml`. When adding a dependency, add it to the `[workspace.dependencies]` table.

## 3. ML Fine-Tuning Workflow (`useragent_id/gm_ml/`)

This workflow is for the "Genetically Modified Machine Learning" (`gm_ml`) experiment. It has specific, non-obvious setup steps.

- **Environment:** The project uses a Python virtual environment located at `useragent_id/gm_ml/`.
    - **Activation:** Before running any Python scripts, activate the environment from the workspace root:
      ```bash
      source useragent_id/gm_ml/bin/activate
      ```

- **Dependencies & CUDA:**
    - Python packages are defined in `useragent_id/gm_ml/requirements.txt`.
    - **CRITICAL:** The environment requires an NVIDIA GPU with the CUDA Toolkit installed. The `flash-attn` package will fail to install without it.
    - **Installation Order Matters:** Due to build dependencies, packages should be installed in a specific order:
      1. `pip install torch`
      2. `pip install --no-build-isolation -r useragent_id/gm_ml/requirements.txt`
    - The `CUDA_HOME` environment variable must be set to the CUDA installation root (e.g., `/usr/local/cuda`) before installing requirements.

- **Workflow Steps:**
    1. **Activate Environment:** `source useragent_id/gm_ml/bin/activate`
    2. **Prepare Dataset:** Run the shell script to gather all `.rs` files for training.
       ```bash
       bash useragent_id/gm_ml/prepare_dataset.sh
       ```
    3. **Run Training:** Execute the main Python script, pointing it to the dataset.
       ```bash
       python3 useragent_id/gm_ml/main.py --dataset_path useragent_id/gm_ml/dataset/rust_crate/
       ```

## 4. Key Files & Conventions

- `/home/scrapedat/WasmAgentTrainer/useragent_id/Cargo.toml`: Defines the Rust workspace and its shared dependencies.
- `/home/scrapedat/WasmAgentTrainer/useragent_id/gm_ml/main.py`: The core script for fine-tuning the SLM.
- `/home/scrapedat/WasmAgentTrainer/useragent_id/gm_ml/prepare_dataset.sh`: The script for collecting the Rust source files into a dataset.
- `/home/scrapedat/WasmAgentTrainer/useragent_id/gm_ml/requirements.txt`: Python package dependencies.
- When working on the Rust crates, look for shared logic in the `/home/scrapedat/WasmAgentTrainer/useragent_id/shared/` directory to avoid duplication.

## 5. Development Practices

### Testing

- To run the entire Rust test suite for the workspace, use the following command from the `useragent_id/` directory:
  ```bash
  cargo test --workspace
  ```
- To run tests for a specific crate, use the `-p` flag. For example, for `user-monitor`:
  ```bash
  cargo test -p user-monitor
  ```

### Code Style

- **Rust:** Please format your code using `rustfmt` before committing. You can run it for the entire workspace from the `useragent_id/` directory:
  ```bash
  cargo fmt --all
  ```
- **Python:** The Python code in `useragent_id/gm_ml/` does not currently have a standardized formatter. Please adhere to PEP 8 guidelines.

---

*** Ideas / Rules from the human user - [Link to the rules file](prompts/rules.prompt.md) ***

- This is a living document and should be updated as the project evolves    

- Encourage collaboration and knowledge sharing among team members.
- Prioritize user feedback and real-world testing to validate assumptions, and iterate on the design based on this feedback.
