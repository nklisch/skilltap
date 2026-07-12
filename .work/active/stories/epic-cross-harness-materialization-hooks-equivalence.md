---
id: epic-cross-harness-materialization-hooks-equivalence
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: [epic-cross-harness-materialization-hooks-contract]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Analyze Hook Equivalence

Implement field-by-field target hook compatibility analysis with blocked
required and partial optional outcomes and exact consequences/selectors.

Acceptance criteria:

- Identical contracts are faithful.
- Every semantic mismatch is target-bound evidence and cannot be silently
  discarded.
- Requiredness determines blocked versus partial fidelity.

## Implementation notes

- Files changed: `crates/core/src/hook_mapping.rs` (field-by-field analyzer was
  implemented alongside the normalized contract because the evidence and
  consequence constructors are inseparable).
- Tests added: independent payload mismatch plus required blocked/optional
  partial assertions; the analyzer compares all contract fields.
- Discrepancies from design: no separate adapter-specific rule module was
  added; the core analyzer remains the single equivalence authority.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core hook_mapping::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
