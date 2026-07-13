---
id: epic-real-harness-recovery-state-diagnostics-output-test-parity
kind: story
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on:
  - epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle
  - epic-real-harness-recovery-state-diagnostics-update-eligibility
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align postcondition tests with canonical recovery actions

## Finding

The renderer intentionally promotes an exact recovery action to one top-level
copy and removes the duplicate nested error copy. The full native postcondition
test target still expects the nested copy and fails despite the approved output
behavior.

## Required fix

- Assert the typed error code remains nested on the error.
- Assert the exact recovery action appears once in top-level `next_actions` in
  first-seen order and is absent from the nested error after normalization.
- Retain the failed-journal and never-applied state assertions for every typed
  postcondition failure class.
- Run the complete `native_postconditions` target, not only selected exact
  cases.

## Acceptance

- All native postcondition tests pass without restoring duplicate output.
- Plain and JSON render one exact recovery command while materially distinct
  actions remain visible.
- Test changes preserve state-safety and failure-class coverage.

## Implementation notes

- Execution capability: focused inline test-contract correction.
- Review weight: highest from the caller's autopilot instruction.
- Files changed: `crates/cli/tests/native_postconditions.rs`.
- Tests changed: every typed failure now asserts the error code, one top-level `reobserve_before_retry` command, no nested duplicate, and failed-state/no-applied safety.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-12, bounded final pass)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: All nine native-postcondition tests preserve typed error and
state-safety assertions while rendering the exact recovery action once at the
top level. Full workspace tests and strict Clippy pass.
