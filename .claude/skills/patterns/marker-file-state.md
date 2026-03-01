# Pattern: Marker-File State

Phase progression tracked by the presence or absence of files on disk — no database, no state machine library.

## Rationale

The orchestrator needs crash-safe, human-inspectable state that survives process restarts and can be manually edited. Marker files achieve all three: they persist across restarts, are visible via `ls`, and can be created/deleted by hand to adjust state.

## Examples

### Example 1: Inner-loop artifact detection
**File**: `src/state.ts:87-159`

The `assessState()` function checks for `VISION.md`, `ROADMAP.md`, `RESEARCH_COMPLETED`, `DESIGN.md`, `VERIFICATION.md`, and other marker files to determine what lifecycle steps are complete for each phase.

### Example 2: Marker creation on step completion
**File**: `src/orchestrator.ts:141`

After a step completes, the orchestrator writes a marker file containing the current ISO timestamp: `writeFile(markerPath, new Date().toISOString())`.

### Example 3: Quality-gate content parsing
**File**: `src/state.ts:141-159`

`QUALITY_GATE.md` and `INTEGRATION-FIX.md` are both marker files AND content files — their presence indicates the step ran, and their content (parsed for `Status: PASS`) indicates whether it succeeded.

## When to Use

- Tracking completion of lifecycle steps in the orchestrator
- Any state that should survive process crashes
- State that humans may need to inspect or manually adjust
