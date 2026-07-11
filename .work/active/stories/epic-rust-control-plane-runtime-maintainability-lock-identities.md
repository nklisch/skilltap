---
id: epic-rust-control-plane-runtime-maintainability-lock-identities
kind: story
stage: done
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

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem.rs` and
  `crates/core/src/runtime/filesystem/locking.rs`.
- Identity restoration: moved the two public traits, two public structs, and
  guard storage fields back to `runtime::filesystem`; removed the child-module
  declarations and parent re-export.
- Implementation boundary: trait implementations, explicit and drop-based
  release, acquisition orchestration, and the nonblocking lock helper remain
  in private `filesystem/locking.rs`.
- Runtime API: `runtime/mod.rs` and all public consumer imports are unchanged.
- `std::any::type_name` probe results:
  - `skilltap_core::runtime::filesystem::SystemConfigurationLock`
  - `skilltap_core::runtime::filesystem::SystemConfigurationLockGuard`
  - `dyn skilltap_core::runtime::filesystem::ConfigurationLock<Guard = skilltap_core::runtime::filesystem::SystemConfigurationLockGuard>`
  - `dyn skilltap_core::runtime::filesystem::ConfigurationLockGuard`
- Rustdoc JSON probe: all four path entries resolve directly to
  `skilltap_core::runtime::filesystem::{ConfigurationLock,ConfigurationLockGuard,SystemConfigurationLock,SystemConfigurationLockGuard}`
  with trait/trait/struct/struct kinds and no child declaration or re-export.
- Size verification: `filesystem.rs` is 386 lines and `locking.rs` is 134;
  both remain within the design target while behavior stays in the child.
- Tests added: none; all 17 adversarial filesystem tests pass unchanged.
- Test identity verification: the pre- and post-correction workspace lists
  contain the same 99 named tests with SHA-256
  `94e0a9b5b01e4a66f8d9a88b24b816f7d51a5cadf302e903c2f3db99f19640a3`.
- Verification passed: external type-name probe, rustdoc JSON path inspection,
  `cargo fmt --all -- --check`,
  `cargo check --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked` (99 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. All four declarations again have canonical
`skilltap_core::runtime::filesystem::*` rustdoc identities; external
`type_name` confirms both structs and the guard associated type. Implementations
and acquisition remain private in the child, the parent is 386 lines, and the
exact 99-test inventory and focused filesystem suite remain green.
