# Claude Block Checker - Quick Usage Guide

## Installation

```bash
# Clone and install
cd claude-block-checker
./install.sh

# Or manually
cargo build --release
cargo install --path .
```

## Basic Usage

### Check all profiles
```bash
claude-block-checker
```

### Check specific profile
```bash
claude-block-checker check cryptic
```

### List available profiles
```bash
claude-block-checker list
```

## Advanced Options

### Detailed view with burn rate
```bash
claude-block-checker --detailed
```

Shows:
- Current token usage rate (tokens/min)
- Cost burn rate ($/hour)
- Projected usage for full 5-hour block

### JSON output
```bash
claude-block-checker --json
```

Perfect for scripting and automation.

### Combine options
```bash
claude-block-checker --detailed --json
```

## Understanding the Output

Each active block shows:
- **Started**: When the 5-hour billing block began
- **Remaining**: Time left in current block
- **Models**: Which Claude models were used
- **Token Usage**: Breakdown by type (input/output/cache)
- **Cost**: Total cost for the block so far

## Profiles

The tool reads from `~/claude-profiles/*/projects/` directories. Each subdirectory under `~/claude-profiles/` is treated as a separate profile.

## Performance

- Processes thousands of JSONL files in milliseconds
- Minimal memory usage
- No network calls required