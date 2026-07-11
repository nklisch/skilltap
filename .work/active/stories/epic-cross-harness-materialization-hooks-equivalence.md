---
id: epic-cross-harness-materialization-hooks-equivalence
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: [epic-cross-harness-materialization-hooks-contract]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Analyze Hook Equivalence

Implement field-by-field target hook compatibility analysis with blocked
required and partial optional outcomes and exact consequences/selectors.

Acceptance criteria:

- Identical contracts are faithful.
- Every semantic mismatch is target-bound evidence and cannot be silently
  discarded.
- Requiredness determines blocked versus partial fidelity.
