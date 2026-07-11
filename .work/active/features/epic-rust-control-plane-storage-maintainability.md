---
id: epic-rust-control-plane-storage-maintainability
kind: feature
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-storage]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Storage Maintainability

## Brief

Reduce structural pressure revealed by the completed storage boundary without
changing behavior, public API identities, serialized bytes, error
classification, test identities, filesystem effects, or platform semantics.

## Evidence

- `storage/managed_artifact.rs` is 609 production lines spanning tree
  validation, error/residual modeling, public contracts, and filesystem
  lifecycle orchestration; its 539-line test sidecar spans three distinct test
  families.
- `storage/repository.rs` is 459 lines and mixes repository ports/filesystem
  orchestration with pure TOML/JSON codec and schema-probing logic.
- `runtime/filesystem/tests.rs` is 599 lines across metadata/no-follow reads,
  publication/copy recovery, ownership/link safety, and configuration locking,
  and no longer mirrors the split production boundary.

The cadence scan explicitly leaves cohesive state schemas, deliberate wire
validation repetition, distinct publication/removal residual models, and
different publication collision flows unchanged.

## Design

This is a mechanical decomposition with a same-toolchain identity baseline.
Public declarations remain in their current parent modules so canonical
rustdoc paths and `type_name` values cannot drift through re-exporting. Private
inherent/trait implementations and helpers move to child modules. Test moves
preserve each existing test name and assertion; no common fake is introduced
unless it makes the behavior more explicit.

First split the managed-artifact and runtime-filesystem test sidecars by
contract, recording the test list. Then move managed-artifact tree validation,
error translation, and repository lifecycle implementations behind the parent
module's unchanged declarations. Separately extract document codecs and schema
probing to a private repository child. Codec tests become directly focused on
classification and deterministic bytes while adapter tests retain filesystem
and concurrency coverage.

## Pre-mortem

- **Re-exports change observable type identity.** Keep every public declaration
  in its original module and compare rustdoc/type-name baselines.
- **Test moves silently weaken coverage.** Compare `cargo test -- --list`
  before and after; names and counts must remain identical.
- **Codec extraction changes error precedence or bytes.** Move logic
  mechanically and retain malformed, duplicate-key, unsupported-version, and
  golden byte assertions at the pure boundary and through repositories.
- **Shared fixtures hide fault behavior.** Keep distinct explicit filesystem
  doubles when their collision or partial-failure semantics differ.

## Implementation units

1. `epic-rust-control-plane-storage-maintainability-managed-tests` — split
   managed-artifact tests into tree contract, real lifecycle/security, and
   failure-mapping modules — depends on `[]`.
2. `epic-rust-control-plane-storage-maintainability-managed-module` — split
   private tree, error-translation, and repository implementations while
   retaining all public declarations in the parent — depends on
   `[epic-rust-control-plane-storage-maintainability-managed-tests]`.
3. `epic-rust-control-plane-storage-maintainability-codecs` — separate private
   document codecs/schema probes and their focused tests from repository
   filesystem orchestration — depends on `[]`.
4. `epic-rust-control-plane-storage-maintainability-runtime-tests` — split the
   runtime filesystem test sidecar by adapter contract — depends on `[]`.

## Acceptance criteria

- Public exports, canonical rustdoc paths, `type_name` values, serialized
  bytes, error classification/order, and filesystem behavior are unchanged.
- Pre/post test lists are identical; every assertion and adversarial scenario
  remains represented.
- Managed-artifact and document-repository private responsibilities are
  navigable in focused child modules without introducing a generic framework.
- No touched production or test module exceeds roughly 400 lines unless a
  cohesive declaration surface makes that limit actively less clear.
- The full locked format/check/Clippy/test/rustdoc ladder passes.
