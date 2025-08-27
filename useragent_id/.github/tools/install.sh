#!/bin/bash

# Install script for phi_helper tool

set -e
echo "Installing phi_helper tool..."

# Check for Python
if ! command -v python3 &>/dev/null; then
    echo "Error: Python 3 is required but not found"
    exit 1
fi

# Install required packages
echo "Installing required Python packages..."
pip install openai

# Create symlink in local bin directory
TOOLS_DIR="$(pwd)"
SCRIPT_PATH="$TOOLS_DIR/phi_helper.py"

# Make the script executable
chmod +x "$SCRIPT_PATH"

# Create user bin directory if it doesn't exist
mkdir -p "$HOME/.local/bin"

# Create symbolic link
ln -sf "$SCRIPT_PATH" "$HOME/.local/bin/phi"
echo "Created symbolic link to phi_helper.py as 'phi' in $HOME/.local/bin"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "Warning: $HOME/.local/bin is not in your PATH"
    echo "Consider adding this line to your .bashrc or .zshrc:"
    echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
echo "Installation complete!"
echo "Run 'phi setup' to configure the tool"
echo "Examples:"
echo "  phi analyze 'How does the agent dispatcher work?' --filter dispatcher"
echo "  phi docs --files src/core/types.rs"
echo "  phi suggest --files src/main.rs"
