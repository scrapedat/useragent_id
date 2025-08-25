# SSH Authentication Setup for Git Operations

This directory contains scripts and tools to set up SSH authentication for Git operations with the UserAgent.ID repository.

## Quick Start

1. **Generate SSH Keys**:
   ```bash
   ./setup-ssh-keys.sh
   ```

2. **Add Public Key to GitHub**:
   - Copy the displayed public key
   - Go to [GitHub SSH Settings](https://github.com/settings/ssh/new)
   - Add the key with a descriptive title

3. **Test SSH Connection**:
   ```bash
   ./test-ssh-connection.sh
   ```

4. **Configure Git to use SSH**:
   ```bash
   ./configure-git-ssh.sh
   ```

5. **Use Git with SSH**:
   ```bash
   ./git-ssh.sh push origin main
   ./git-ssh.sh pull origin main
   ```

## Manual Commands

If you prefer to run Git commands manually with SSH authentication:

```bash
# Set remote URL to SSH
GIT_SSH_COMMAND='ssh -i .ssh/id_ed25519 -o IdentitiesOnly=yes' git remote set-url origin git@github.com:scrapedat/useragent_id.git

# Push to main branch  
GIT_SSH_COMMAND='ssh -i .ssh/id_ed25519 -o IdentitiesOnly=yes' git push -u origin main

# Pull from main branch
GIT_SSH_COMMAND='ssh -i .ssh/id_ed25519 -o IdentitiesOnly=yes' git pull origin main
```

## Troubleshooting

### "Permission denied (publickey)" Error

This error occurs when:
1. The SSH key hasn't been added to your GitHub account
2. The SSH key file doesn't exist or has wrong permissions
3. The key isn't being used correctly

**Solutions**:
1. Ensure you've added your public key to GitHub: `cat .ssh/id_ed25519.pub`
2. Check file permissions: `ls -la .ssh/`
3. Test the connection: `./test-ssh-connection.sh`

### "Host key verification failed" Error

This happens when GitHub's host key isn't in your known_hosts file.

**Solution**:
Add GitHub to known hosts:
```bash
ssh-keyscan github.com >> .ssh/known_hosts
```

### Key Not Found Error

If you see "SSH key not found" errors:
1. Run `./setup-ssh-keys.sh` to generate keys
2. Ensure you're running scripts from the repository root directory

## Security Notes

- Private SSH keys (`.ssh/id_*`) are automatically excluded from Git via `.gitignore`
- Never commit private keys to version control
- Keep your SSH keys secure and don't share them
- Use strong passphrases for additional security (optional)

## File Structure

```
.ssh/
├── .gitkeep              # Ensures .ssh directory is tracked
├── id_ed25519           # Private key (auto-generated, not committed)
├── id_ed25519.pub       # Public key (auto-generated, not committed)
└── known_hosts          # GitHub host keys (auto-generated)

setup-ssh-keys.sh        # Generate SSH keys
test-ssh-connection.sh    # Test GitHub SSH connection  
configure-git-ssh.sh     # Switch Git to use SSH
git-ssh.sh              # Wrapper for Git commands with SSH
```