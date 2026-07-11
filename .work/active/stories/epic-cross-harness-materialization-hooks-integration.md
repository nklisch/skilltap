---
id: epic-cross-harness-materialization-hooks-integration
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: [epic-cross-harness-materialization-hooks-equivalence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Hand Hook Analysis Into Reconciliation

Expose pure hook compatibility through reconciliation/materialization planning
without native registration or managed filesystem writes.

Acceptance criteria:

- Required hook mismatch blocks before publication.
- Optional hook loss is represented by exact partial selectors.
- Scope-bearing resource/component identity is preserved end to end.
