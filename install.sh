#!/bin/bash
set -e

echo "Building claude-block-checker..."
cargo build --release

echo ""
echo "Installing to ~/.cargo/bin..."
cargo install --path .

echo ""
echo "Installation complete!"
echo ""
echo "You can now use 'claude-block-checker' from anywhere."
echo ""
echo "Examples:"
echo "  claude-block-checker                  # Check all profiles"
echo "  claude-block-checker list             # List available profiles"
echo "  claude-block-checker check cryptic    # Check specific profile"
echo "  claude-block-checker --detailed       # Show burn rate and projections"
echo "  claude-block-checker --json           # Output as JSON"