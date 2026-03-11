#!/bin/bash
set -e

echo "=== deltav Devcontainer Post-Create Setup ==="

# Verify Rust toolchain
echo "Verifying Rust toolchain..."
echo "  rustc: $(rustc --version)"
echo "  cargo: $(cargo --version)"
echo "  wasm32 target: $(rustup target list --installed | grep wasm32 || echo 'not found')"

# Verify dev tools
echo "Verifying dev tools..."
echo "  cargo-leptos: $(cargo leptos --version 2>/dev/null || echo 'not found')"
echo "  cargo-watch: $(cargo watch --version 2>/dev/null || echo 'not found')"
echo "  cargo-deny: $(cargo deny --version 2>/dev/null || echo 'not found')"
echo "  cargo-audit: $(cargo audit --version 2>/dev/null || echo 'not found')"

# Verify Claude Code CLI
echo "Verifying Claude Code CLI..."
echo "  claude: $(claude --version 2>/dev/null || echo 'not found')"

# Check for API keys
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "ANTHROPIC_API_KEY detected - Claude Code ready to use"
else
    echo "ANTHROPIC_API_KEY not set - run 'claude' and use /login to authenticate"
fi

if [ -n "$GITHUB_TOKEN" ]; then
    echo "GITHUB_TOKEN detected - GitHub API access ready"
else
    echo "GITHUB_TOKEN not set - needed for deltav GitHub API calls"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Available tools:"
echo "  cargo, rustc, cargo-leptos, cargo-watch (Rust web development)"
echo "  cargo-deny, cargo-audit (dependency security)"
echo "  claude (AI assistant)"
echo ""
echo "Quick start:"
echo "  cargo build              # Build the project"
echo "  cargo leptos serve       # Serve with hot-reload (port 3000)"
echo "  cargo watch -x run       # Auto-reload on changes"
echo ""
