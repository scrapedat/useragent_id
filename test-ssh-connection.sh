#!/bin/bash

# Test SSH Connection to GitHub
# This script tests the SSH connection to GitHub using the generated key

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_DIR="${SCRIPT_DIR}/.ssh"
KEY_PATH="${SSH_DIR}/id_ed25519"

echo "🔗 Testing SSH Connection to GitHub"
echo "===================================="

# Check if key exists
if [ ! -f "$KEY_PATH" ]; then
    echo "❌ SSH key not found at $KEY_PATH"
    echo "   Run ./setup-ssh-keys.sh first to generate the key"
    exit 1
fi

# Test SSH connection
echo "🧪 Testing SSH connection..."
if GIT_SSH_COMMAND="ssh -i $KEY_PATH -o IdentitiesOnly=yes -o StrictHostKeyChecking=no" ssh -T git@github.com 2>&1 | grep -q "successfully authenticated"; then
    echo "✅ SSH connection successful!"
    echo "   You can now use Git with SSH authentication"
else
    echo "❌ SSH connection failed"
    echo "   Make sure you've added your public key to GitHub:"
    echo "   cat $KEY_PATH.pub"
    echo ""
    echo "   Add it to: https://github.com/settings/ssh/new"
    exit 1
fi