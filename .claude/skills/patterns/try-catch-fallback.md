# Pattern: Try-Catch-Fallback

Error handling returns null, false, or a default value instead of throwing — never throw for recoverable operations.

## Rationale

The orchestrator must be resilient to transient failures (missing files, failed builds, subprocess crashes). Throwing exceptions would require catch blocks at every call site and complicate the control flow. By returning fallback values, callers can use simple if-checks and the orchestrator loop continues cleanly.

## Examples

### Example 1: fileExists()
**File**: `src/state.ts:509-516`

Tries `access()`, catches any error and returns `false`. The canonical filesystem-probe pattern used pervasively throughout state assessment.

### Example 2: readFileOr()
**File**: `src/state.ts:536-542`

Tries `readFile()`, returns a fallback string on any error. No error distinction — all failures are equivalent to "file doesn't exist yet."

### Example 3: commandSucceeds()
**File**: `src/state.ts:544-550`

Runs `exec(cmd)`, catches any error and returns `false`. Build and test pass/fail are modeled as boolean success.

### Example 4: runAgent()
**File**: `src/cli.ts:72-159`

Wraps the entire subprocess lifecycle in a `new Promise`, resolving (never rejecting) with `{ success: false, error: '...' }` for every failure path.

## When to Use

- Filesystem operations that check for file existence
- Subprocess invocations where failure is a normal outcome
- Any operation where the caller needs a yes/no answer, not error details
