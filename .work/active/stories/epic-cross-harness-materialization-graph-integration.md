---
id: epic-cross-harness-materialization-graph-integration
kind: story
stage: review
tags: []
parent: epic-cross-harness-materialization-graph
depends_on: [epic-cross-harness-materialization-graph-readers]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Integrate Graph Evidence With Materialization Planning

Wire the reader and normalizer into `crates/core/src/materialization.rs` (and
the composition root that supplies readers) without adding target selection or
acknowledgment behavior prematurely.

Acceptance criteria:

- Reader failures stop planning before inventory, state, or managed-artifact
  writes.
- Successful graphs preserve required/optional dependency semantics when
  passed to the existing materialization planner.
- Repeating an unchanged read-and-plan handoff is deterministic and produces
  no writes.

## Implementation notes

- Files changed: `crates/core/src/materialization.rs`.
- Tests added: deterministic graph handoff/planning and reader-failure tests.
- Discrepancies from design: the handoff returns a typed
  `GraphPlanningError` and a separate `plan_source_materialization` helper;
  target support remains an explicit downstream input to keep graph reads
  target-independent.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core materialization::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.
