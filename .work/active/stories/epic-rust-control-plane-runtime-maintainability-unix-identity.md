---
id: epic-rust-control-plane-runtime-maintainability-unix-identity
kind: story
stage: review
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Unix Identity Internals

Move private file identity, no-follow open, and descriptor/path verification
helpers to `runtime/filesystem/unix_identity.rs`, keeping cfg pairs adjacent,
call order and error mapping identical, and all public exports unchanged. Run
the adversarial filesystem suite and full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem.rs` and new private
  `crates/core/src/runtime/filesystem/unix_identity.rs`.
- Extraction: moved `FileIdentity`, no-follow file/directory/lock open helpers,
  descriptor and path identity helpers, and identity verification helpers into
  the private sidecar with Unix/non-Unix implementations adjacent.
- Behavior preservation: parent call sites, call order, native flags, error
  mapping, publication cleanup, lock behavior, and public exports are unchanged.
- Tests added: none; the 17-test adversarial filesystem suite passes unchanged.
- Test identity verification: the live baseline and post-extraction core lists
  contain the same 98 named tests with SHA-256
  `0e01801761bf1688083f1e92f9368ee9994b707eca458b13b0dea711e91f9d67`.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked` (99 workspace tests: 98 core and 1 test-support),
  and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`.
- Discrepancies from design: the current core baseline is 98 tests rather than
  the stale 99-test context; the exact live inventory was preserved.
- Adjacent issues parked: none.
