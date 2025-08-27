# WasmAgentTrainer Development Tools

This directory contains tools to assist with development of the WasmAgentTrainer project.

## Phi Helper Tool

The `phi_helper.py` tool integrates Phi-3 as an AI assistant for your codebase. It can analyze code, generate documentation, and suggest improvements.

### Installation

1. Make sure you have Python 3 and pip installed
2. Run the installation script:

```bash
cd .github/tools
./install.sh
```

3. Configure the tool:

```bash
phi setup
```

You'll need to provide an OpenAI API key or Azure OpenAI endpoint with access to the Phi-3 model.

### Usage Examples

Analyze code:
```bash
# Analyze all project files
phi analyze "How does the agent dispatcher work?"

# Filter specific files
phi analyze "Explain the main execution flow" --filter main.rs

# Specify exact files
phi analyze "How do these files interact?" --files src/main.rs src/core/types.rs
```

Generate documentation:
```bash
# Generate docs for specific files
phi docs --files src/core/types.rs src/core/mod.rs --output docs/core.md
```

Get improvement suggestions:
```bash
phi suggest --files src/wasm/orchestrator.rs
```

### Configuration

Your configuration is stored in `~/.config/phi_helper/config.json`. 
You can modify it directly or run `phi setup` again to reconfigure.

## Additional Tools

More development tools may be added to this directory in the future.
