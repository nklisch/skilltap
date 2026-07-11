---
id: epic-rust-control-plane-runtime-maintainability-lock-identities
kind: story
stage: implementing
tags: [refactor, correctness]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-locking-module]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Restore Lock Canonical Identities

Move the four public lock trait/struct declarations and guard storage back to
`runtime::filesystem`, leaving their impls and private acquisition helpers in
`filesystem/locking.rs`.

## Acceptance criteria

- Rustdoc JSON and `std::any::type_name` report the same canonical
  `skilltap_core::runtime::filesystem::*` identities as the pre-refactor
  baseline for all four public lock items.
- Existing `skilltap_core::runtime::*` consumer imports and behavior remain
  unchanged; no duplicate public declaration or child re-export remains.
- Parent production size stays near the design target and the child continues
  to own lock implementation/acquisition logic.
- Exact test inventory and full locked format/check/Clippy/test/rustdoc ladder
  pass.
