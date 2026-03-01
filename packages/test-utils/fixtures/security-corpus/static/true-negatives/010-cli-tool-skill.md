---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean CLI tool building skill"
---
# CLI Tool Patterns

Build command-line tools with good UX.

## Argument Parsing

Define commands, flags, and positional arguments declaratively.
Provide help text for every option. Support both short and long flags.

## Output

- Use stdout for data, stderr for diagnostics
- Support `--json` for machine-readable output
- Use exit code 0 for success, 1 for errors
- Show progress for long operations

## Configuration

Load config from (in priority order):
1. CLI flags
2. Environment variables
3. Config file (user config directory)
4. Built-in defaults

## Error Messages

Show what went wrong, why, and how to fix it.
Include the failing input when safe to do so.
