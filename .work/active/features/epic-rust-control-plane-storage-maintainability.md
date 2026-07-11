---
id: epic-rust-control-plane-storage-maintainability
kind: feature
stage: drafting
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

<!-- The refactor design pass will define implementation units. -->
