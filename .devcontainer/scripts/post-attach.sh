#!/bin/bash
set -e

echo "=== deltav Post-Attach Setup ==="

# Mark workspace and skill directories as trusted in git
echo "Configuring git safe directories..."
git config --global --add safe.directory /workspaces/deltav
git config --global --add safe.directory /workspaces/deltav/.claude/skills/claude-skill-rust

# Install latest syskit from master branch
echo "Installing latest syskit..."
cd /workspaces/deltav
curl -fsSL https://raw.githubusercontent.com/londey/syskit/refs/heads/master/install_syskit.sh | bash
echo "syskit installed successfully"

echo "=== Post-Attach Setup Complete ==="
