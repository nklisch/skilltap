---
id: story-reconciliation-instruction-repair
kind: story
stage: implementing
tags: [correctness, testing]
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile instruction bridge drift through sync

## Finding

The final completion review found that reconciliation selects
`InstructionLocation` resources but the sync branch emits an unconditional
`no_change` operation. The explicit instructions setup/repair adapter already
owns bridge publication, while the synchronization contract includes
instruction repair.

## Required behavior

- `plan` reports instruction bridge repair/no-op based on fresh bridge state.
- `sync` delegates selected instruction resources through the existing setup /
  repair adapter with the same acknowledgment safety rules.
- Drifted or missing managed bridges are repaired; unmanaged or conflicting
  user-authored content remains blocked unless explicitly acknowledged.
- Add isolated compiled coverage for drift, repair, repeat no-change, and
  project/target selection.

## Implementation notes

This is a final-review finding promoted directly to an implementation story;
the foundation instruction-reconciliation contract remains authoritative.

Implemented in `4369a0a`:

- `plan` now inspects canonical and harness bridge paths and reports
  `no_change`, `repair`, or blocked conflict operations from fresh filesystem
  state.
- `sync` delegates instruction resources through the existing setup/repair
  execution journal with a target-filtered adapter, preserving canonical
  inventory records when reconciling a different selected harness.
- `--yes` is required for divergent bridge replacement and duplicate cleanup;
  missing managed bridges remain safely repairable without acknowledgment.
- Compiled coverage exercises drift planning, blocked sync, acknowledged
  repair, repeat no-change, global target isolation, and project/Claude scope
  selection.

Verification: `cargo fmt --all`, focused compiled instruction/reconciliation
tests, `cargo clippy --workspace --all-targets --offline -- -D warnings`, and
the full workspace test suite passed.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `story-reconciliation-instruction-repair-nested-plan`
**Important**: none
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a focused
correctness/foundation-contract pass. Existing compiled instruction tests and
the full workspace suite passed. The implementation is otherwise aligned with
the documented generic `--yes` and target/scope behavior, but plan must model
the supported nested-only project Claude bridge before this story can close.
