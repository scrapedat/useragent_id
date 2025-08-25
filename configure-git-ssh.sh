#!/bin/bash

# Configure Git to use SSH instead of HTTPS
# This script switches the repository to use SSH for Git operations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_DIR="${SCRIPT_DIR}/.ssh"
KEY_PATH="${SSH_DIR}/id_ed25519"

echo "⚙️  Configuring Git to use SSH"
echo "==============================="

# Check if key exists
if [ ! -f "$KEY_PATH" ]; then
    echo "❌ SSH key not found at $KEY_PATH"
    echo "   Run ./setup-ssh-keys.sh first to generate the key"
    exit 1
fi

# Get current remote URL
CURRENT_URL=$(git remote get-url origin)
echo "📍 Current remote URL: $CURRENT_URL"

# Convert HTTPS URL to SSH if needed
if [[ "$CURRENT_URL" == https://github.com/* ]]; then
    SSH_URL=$(echo "$CURRENT_URL" | sed 's|https://github.com/|git@github.com:|')
    echo "🔄 Converting to SSH URL: $SSH_URL"
    
    # Set the new remote URL
    GIT_SSH_COMMAND="ssh -i $KEY_PATH -o IdentitiesOnly=yes" git remote set-url origin "$SSH_URL"
    echo "✅ Remote URL updated to use SSH"
elif [[ "$CURRENT_URL" == git@github.com:* ]]; then
    echo "✅ Already using SSH URL"
else
    echo "⚠️  Unknown URL format: $CURRENT_URL"
    echo "   Please manually set the SSH URL:"
    echo "   GIT_SSH_COMMAND='ssh -i $KEY_PATH -o IdentitiesOnly=yes' git remote set-url origin git@github.com:scrapedat/useragent_id.git"
    exit 1
fi

# Display the final configuration
echo ""
echo "🎉 Git SSH Configuration Complete!"
echo "==================================="
echo "Remote URL: $(git remote get-url origin)"
echo ""
echo "💡 To use Git with SSH authentication, prefix commands with:"
echo "   GIT_SSH_COMMAND='ssh -i $KEY_PATH -o IdentitiesOnly=yes'"
echo ""
echo "📝 Examples:"
echo "   GIT_SSH_COMMAND='ssh -i $KEY_PATH -o IdentitiesOnly=yes' git push origin main"
echo "   GIT_SSH_COMMAND='ssh -i $KEY_PATH -o IdentitiesOnly=yes' git pull origin main"
echo ""
echo "🚀 Or use the convenience script:"
echo "   ./git-ssh.sh push origin main"
echo "   ./git-ssh.sh pull origin main"