# Pattern: Quality-Gate Remediation

Selective marker deletion resets specific lifecycle stages on failure, allowing targeted re-execution rather than full phase restart.

## Rationale

When the quality gate fails, the failure type determines which lifecycle steps need to be re-run. A test failure doesn't require re-designing — only re-verifying. A vision-level failure requires reimplementation. Selective marker deletion implements this graduated retry by removing only the markers for affected stages.

## Examples

### Example 1: Refactor remediation
**File**: `src/orchestrator.ts:376-393`

Clears outer-loop markers only (`REFACTOR-PLAN.md` through `QUALITY_GATE.md`), preserving inner-loop implementation and verification.

### Example 2: Tests remediation
**File**: `src/orchestrator.ts:395-412`

Clears verification markers and outer-loop markers but preserves `IMPLEMENTED` — the code stays, only verification re-runs.

### Example 3: Vision (full) remediation
**File**: `src/orchestrator.ts:413-445`

Full reset: clears `IMPLEMENTED`, `VERIFICATION.md`, and all outer-loop markers. The phase re-implements from the design.

## When to Use

- Quality-gate failure handling in the orchestrator
- Any situation where partial state reset is more efficient than full restart
- The remediation type is determined by parsing `QUALITY_GATE.md` content
