#!/bin/bash

# SSH Key Setup Script for UserAgent.ID Repository
# This script generates SSH keys and helps configure Git for SSH authentication

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_DIR="${SCRIPT_DIR}/.ssh"
KEY_NAME="id_ed25519"
KEY_PATH="${SSH_DIR}/${KEY_NAME}"

echo "🔑 SSH Key Setup for UserAgent.ID Repository"
echo "============================================="

# Create SSH directory if it doesn't exist
if [ ! -d "$SSH_DIR" ]; then
    echo "📁 Creating SSH directory..."
    mkdir -p "$SSH_DIR"
    chmod 700 "$SSH_DIR"
fi

# Check if key already exists
if [ -f "$KEY_PATH" ]; then
    echo "⚠️  SSH key already exists at $KEY_PATH"
    echo "   If you want to regenerate it, delete the existing key first:"
    echo "   rm $KEY_PATH $KEY_PATH.pub"
    exit 1
fi

# Generate SSH key
echo "🔐 Generating Ed25519 SSH key..."
read -p "Enter your email address for the SSH key: " email

ssh-keygen -t ed25519 -C "$email" -f "$KEY_PATH" -N ""

# Set proper permissions
chmod 600 "$KEY_PATH"
chmod 644 "$KEY_PATH.pub"

echo "✅ SSH key generated successfully!"
echo ""
echo "📋 Next steps:"
echo "1. Copy your public key to GitHub:"
echo "   cat $KEY_PATH.pub"
echo ""
echo "2. Add this key to your GitHub account:"
echo "   - Go to GitHub Settings > SSH and GPG keys"
echo "   - Click 'New SSH key'"
echo "   - Paste the public key content"
echo ""
echo "3. Test the connection:"
echo "   ./test-ssh-connection.sh"
echo ""
echo "4. Configure Git to use SSH:"
echo "   ./configure-git-ssh.sh"
echo ""

# Display the public key
echo "🔑 Your public key (copy this to GitHub):"
echo "=========================================="
cat "$KEY_PATH.pub"
echo ""