# Claude Block Checker

A focused Rust implementation for checking Claude Code usage in the current billing block across multiple profiles.

## Features

- Fast, native performance
- Checks all profiles under `~/claude-profiles`
- Shows active 5-hour billing blocks
- Token usage breakdown (input/output/cache)
- Cost calculation based on model pricing
- Time remaining in current block
- **Time until usage limit** - Shows how long until 300M token limit
- **Recommends best profile** - Automatically suggests profile with most headroom
- Burn rate and projections
- Colored terminal output
- JSON output support

## Installation

### Build from source

```bash
cd claude-block-checker
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .

# Or copy binary manually
cp target/release/claude-block-checker /usr/local/bin/
```

### Quick build

```bash
# Build optimized binary
cargo build --release

# Run directly
./target/release/claude-block-checker
```

## Usage

```bash
# Check all profiles (default)
claude-block-checker

# Check specific profile
claude-block-checker --profile cryptic

# Check with detailed output (burn rate, projections)
claude-block-checker --detailed

# Output as JSON
claude-block-checker --json

# List available profiles
claude-block-checker list

# Check specific profile with subcommand
claude-block-checker check cryptic
```

## Commands

- `claude-block-checker` - Check all profiles (default)
- `claude-block-checker all` - Explicitly check all profiles
- `claude-block-checker list` - List available profiles
- `claude-block-checker check [PROFILE]` - Check specific profile

## Options

- `-p, --profile <NAME>` - Check specific profile
- `-d, --detailed` - Show detailed breakdown with burn rates
- `-j, --json` - Output in JSON format
- `-h, --help` - Show help

## Output

### Default output
```
Claude Code Usage - Current Block Report
Time: 2024-01-15 10:30:45 UTC
Found 3 profiles

━━━ Profile: cryptic ━━━
  ● Active Block
  Started: 2024-01-15 08:00:00 UTC
  Remaining: 2h 30m
  Models: claude-3-5-sonnet-20241022

  Token Usage:
    Input:  45,320
    Output: 12,850
    Total:  58,170

  Cost: $0.001234

━━━ Summary ━━━
Active profiles: 3/3

Recommended Profile:
  cryptic-1 → 8h 45m until limit
```

### Detailed output (--detailed)
Adds:
- Burn rate (tokens/min, $/hour)
- **Time until 300M token limit** with color coding:
  - 🟢 Green: >3 hours remaining
  - 🟡 Yellow: 1-3 hours remaining  
  - 🔴 Red: <1 hour remaining
- Percentage of limit used
- Projected usage for full 5-hour block

### JSON output (--json)
```json
{
  "profiles": [
    {
      "name": "cryptic",
      "active_block": {
        "start_time": "2024-01-15T08:00:00Z",
        "end_time": "2024-01-15T13:00:00Z",
        "is_active": true,
        "total_tokens": 58170,
        "total_cost": 0.001234,
        ...
      }
    }
  ],
  "summary": {
    "total_profiles": 3,
    "active_profiles": 2,
    "total_tokens": 125000,
    "total_cost": 0.0025
  }
}
```

## Performance

- Written in Rust for maximum performance
- Processes thousands of JSONL files in milliseconds
- Minimal memory footprint
- No external API calls needed

## Configuration

The tool reads directly from `~/claude-profiles/*/projects/**/*.jsonl` files. No configuration needed.

### Model Pricing

Default pricing is embedded in the binary. Currently supports:
- Claude 3.5 Sonnet
- Claude 3.5 Haiku  
- Claude 3 Opus
- Claude 3 Sonnet
- Claude 3 Haiku
- Claude 4 models

## Development

```bash
# Run tests
cargo test

# Run with verbose output
RUST_LOG=debug cargo run

# Build for different targets
cargo build --target x86_64-apple-darwin --release
cargo build --target aarch64-apple-darwin --release
```

## Why Rust?

- **Speed**: 10-100x faster than interpreted languages
- **Safety**: Memory safe with no garbage collector
- **Efficiency**: Low memory usage, instant startup
- **Reliability**: Strong type system catches errors at compile time
- **Portability**: Single binary with no dependencies