---
id: epic-cross-harness-materialization-compatibility-integration
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-compatibility
depends_on: [epic-cross-harness-materialization-compatibility-aggregate]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
