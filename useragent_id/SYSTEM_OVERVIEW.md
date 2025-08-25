# System Overview: WasmAgentTrainer

## Purpose
WasmAgentTrainer is a modular automation platform designed for human users to automate tasks through a simple user interface (UI). The backend consists of four main agent modules that work together to observe, learn, train, and execute automations based on user actions.

## Workflow
1. **User Monitor** (`useragent_id/apps/user-monitor`)
   - Records and monitors user actions via the UI.
   - Captures behavioral data for analysis.

2. **Task Learner** (`useragent_id/apps/task-learner`)
   - Analyzes recorded actions to identify patterns and tasks.
   - Prepares training data for agent models.

3. **Agent Trainer** (`useragent_id/apps/agent-trainer`)
   - Trains agent models using the learned patterns and training data.
   - Updates and manages agent models for automation.

4. **Agent Runner** (`useragent_id/apps/agent-runner`)
   - Executes automations using the trained agent models.
   - Delivers results and feedback to the UI for the user.

## User Experience
- Human users interact only with the UI.
- All backend logic and automation is handled by the four agent modules in sequence.
- The system is designed to be accessible to non-technical users, enabling them to automate tasks without writing code.

## Architecture
- **Modular Design:** Each agent module is independent and focused on a specific stage of the automation workflow.
- **Shared Utilities:** Common types, configuration, and logic are stored in `useragent_id/shared` for maintainability and extensibility.

## Summary
WasmAgentTrainer empowers users to automate tasks by simply using the UI. The system observes user actions, learns patterns, trains intelligent agents, and executes automations—all without requiring technical expertise from the user.

## Project Status (As of August 23, 2025)

- **Phase 1: Core Types & Blueprints - ✅ Complete**
    - `shared`: Core data types (`Recording`, `LearnedTask`, `Agent`) are defined.
    - **Module READMEs**: Each app has a `README.md` defining its role, inputs, and outputs.

- **Phase 2: Core Logic Implementation**
    - `user-monitor`: ✅ **Complete**. The UI can record user keyboard/mouse actions and save them to a file.
    - `task-learner`: ⏳ **In Progress**. Implementing the logic to load recordings and transform them into `LearnedTask`s.
    - `agent-trainer`: ⬜️ **Not Started**.
    - `agent-runner`: ⬜️ **Not Started**.

- **Phase 3: UI & Integration**
    - **Main UI**: ⬜️ **Not Started**.
