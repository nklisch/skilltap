---
id: epic-rust-control-plane-storage-maintainability-managed-tests
kind: story
stage: done
tags: [refactor, testing]
parent: epic-rust-control-plane-storage-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Split Managed Artifact Tests by Contract

Mechanically split the managed-artifact sidecar into tree-contract,
real-filesystem lifecycle/security, and failure-mapping modules with minimal
shared fixtures. Preserve test identities, assertions, and explicit fault
doubles. Record and compare the test list, then run the full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/storage/managed_artifact/tests.rs` now owns
  only the minimal shared imports and fixtures; tree-contract,
  lifecycle/security, and failure-mapping coverage lives in focused child
  source files beneath `managed_artifact/tests/`.
- Tests added: none. All nine existing tests, their fully qualified identities,
  assertions, and explicit filesystem doubles are unchanged. Direct inclusion
  keeps the canonical `storage::managed_artifact::tests::*` identities instead
  of introducing nested module paths.
- Verification: the complete `cargo test --locked -p skilltap-core -- --list`
  output is identical before and after. The focused nine-test suite and the
  locked workspace format, all-target check, warnings-denied Clippy, 150-test
  workspace suite including doctests, and warnings-denied rustdoc all pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. The lexical split preserves all nine fully qualified test identities,
assertions, explicit fakes, and the complete locked test list.
