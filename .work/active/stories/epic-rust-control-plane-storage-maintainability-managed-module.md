---
id: epic-rust-control-plane-storage-maintainability-managed-module
kind: story
stage: review
tags: [refactor]
parent: epic-rust-control-plane-storage-maintainability
depends_on: [epic-rust-control-plane-storage-maintainability-managed-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Decompose Managed Artifact Implementation

Move private tree validation, runtime-error translation, and repository
lifecycle implementations into focused child modules while retaining every
public declaration in `storage::managed_artifact`. Preserve canonical rustdoc
and `type_name` identities, behavior, errors, and test list. Run the full
locked ladder and compare the identity baseline.

## Implementation notes

- Files changed: `crates/core/src/storage/managed_artifact.rs` retains every
  public declaration; private tree validation, runtime-error translation, and
  concrete repository lifecycle implementations now live in focused
  `tree_validation.rs`, `error_translation.rs`, and `repository.rs` children.
  `managed_artifact/tests.rs` now imports `RuntimeError` explicitly instead of
  inheriting the parent module's removed private import.
- Tests added: none. The complete `skilltap-core` test-name list and all nine
  managed-artifact test identities, bodies, and assertions are unchanged.
- Identity verification: a clean `HEAD` build and the refactored build emitted
  identical `std::any::type_name` values for all twelve public managed-artifact
  types/traits. The pre/post rustdoc path list and ordered public declaration
  list are also identical.
- Verification: focused managed-artifact tests pass; the locked workspace
  format, all-target check, warnings-denied Clippy, 157-test workspace suite
  including integration tests and doctests, and warnings-denied rustdoc ladder
  pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.
