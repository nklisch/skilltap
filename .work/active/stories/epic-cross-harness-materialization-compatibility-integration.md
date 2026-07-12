---
id: epic-cross-harness-materialization-compatibility-integration
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-compatibility
depends_on: [epic-cross-harness-materialization-compatibility-aggregate]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Expose Compatibility Through Reconciliation

Wire the pure analyzer into `crates/core/src/reconciliation.rs` while keeping
scope-bearing selectors and no-write classification boundaries intact.

Acceptance criteria:

- Reconciliation consumes the analyzer's aggregate without rebuilding evidence
  or consequences.
- Project and global resource keys remain exact in component selectors.
- Faithful, partial, blocked, and conflict paths are covered by integration
  tests without native or managed filesystem mutation.

## Implementation notes

- Files changed: `crates/core/src/reconciliation.rs`.
- Tests added: scope-exact partial selector integration through the
  reconciliation boundary.
- Discrepancies from design: the boundary is a pure forwarding function to the
  analyzer; operation-class construction remains with later materialization
  features so this story does not duplicate planning policy.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core reconciliation::tests::reconciliation_exposes_scope_exact_compatibility_selectors --offline`
  — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
