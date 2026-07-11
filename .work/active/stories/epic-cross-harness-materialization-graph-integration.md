---
id: epic-cross-harness-materialization-graph-integration
kind: story
stage: implementing
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
