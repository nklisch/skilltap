# Pattern: Stream-JSON Subprocess

External process spawned with piped stdout, parsed as newline-delimited JSON.

## Rationale

The Claude CLI outputs structured events as NDJSON when invoked with `--output-format stream-json`. Spawning it as a subprocess with piped stdout allows the orchestrator to receive real-time tool-use events, text output, and completion signals without waiting for the process to finish.

## Examples

### Example 1: Process spawning with piped I/O
**File**: `src/cli.ts:73-77`

`spawn(binary, args, { stdio: ['ignore', 'pipe', 'pipe'] })` — stdin closed, stdout and stderr piped for line-by-line parsing.

### Example 2: Line-buffered NDJSON parsing
**File**: `src/cli.ts:201-230`

`createLineBuffer()` is a closure returning `{ push, flush }` — it accumulates data chunks and emits complete lines, handling partial JSON that arrives across chunk boundaries.

### Example 3: Event type dispatch
**File**: `src/cli.ts:232-293`

`parseStreamLine()` handles 8+ different JSON event shapes from the Claude CLI (assistant messages, tool use, tool results, stream events, content blocks) with field-name fallbacks for protocol compatibility.

## When to Use

- Invoking the Claude CLI as a subprocess
- Any external process that outputs structured events on stdout
- Situations requiring real-time event streaming from a long-running process
