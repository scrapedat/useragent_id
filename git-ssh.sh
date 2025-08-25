#!/bin/bash

# Git SSH Wrapper Script
# This script wraps git commands to automatically use SSH authentication

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_DIR="${SCRIPT_DIR}/.ssh"
KEY_PATH="${SSH_DIR}/id_ed25519"

# Check if key exists
if [ ! -f "$KEY_PATH" ]; then
    echo "❌ SSH key not found at $KEY_PATH"
    echo "   Run ./setup-ssh-keys.sh first to generate the key"
    exit 1
fi

# Check if any arguments were provided
if [ $# -eq 0 ]; then
    echo "📖 Git SSH Wrapper Usage"
    echo "========================"
    echo "This script wraps git commands to use SSH authentication automatically."
    echo ""
    echo "Usage: $0 <git-command> [arguments...]"
    echo ""
    echo "Examples:"
    echo "  $0 push origin main"
    echo "  $0 pull origin main" 
    echo "  $0 status"
    echo "  $0 log --oneline"
    echo ""
    echo "🔑 Using SSH key: $KEY_PATH"
    exit 0
fi

# Execute git command with SSH configuration
GIT_SSH_COMMAND="ssh -i $KEY_PATH -o IdentitiesOnly=yes" git "$@"