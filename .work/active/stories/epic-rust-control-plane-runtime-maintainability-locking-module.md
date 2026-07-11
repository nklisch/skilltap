---
id: epic-rust-control-plane-runtime-maintainability-locking-module
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on:
  - epic-rust-control-plane-runtime-maintainability-unix-identity
  - epic-rust-control-plane-runtime-maintainability-publication-module
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Configuration Locking

Move the configuration lock port, system adapter, guard, acquisition helpers,
and release behavior to `runtime/filesystem/locking.rs`. Preserve public names,
nonblocking/RAII semantics, lock ordering, identity checks, error precedence,
and all adversarial identities. Remove only now-dead parameters/imports and run
the full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem.rs` and new private
  `crates/core/src/runtime/filesystem/locking.rs`.
- Extraction: moved the configuration lock port, system adapter, guard,
  explicit and drop-based release behavior, acquisition orchestration, and
  nonblocking file-lock helper into the locking sidecar.
- Public surface: `filesystem.rs` explicitly re-exports
  `ConfigurationLock`, `ConfigurationLockGuard`, `SystemConfigurationLock`,
  and `SystemConfigurationLockGuard`; `runtime/mod.rs` and consumers remain
  unchanged.
- Test seam preservation: `try_acquire_with` remains privately available to
  the existing lock path-swap test through a test-only parent import.
- Behavior preservation: directory-before-file acquisition, no-follow opens,
  descriptor/path identity checks, nonblocking contention, error precedence,
  explicit release, and RAII release order are unchanged.
- Cleanup: removed only lock-specific imports made dead in `filesystem.rs` by
  the move; no unrelated parameters or behavior were changed.
- Size verification: production modules are 370 lines (`filesystem.rs`), 151
  (`locking.rs`), 269 (`publication.rs`), and 166 (`unix_identity.rs`).
- Tests added: none; all 17 adversarial filesystem tests pass unchanged.
- Test identity verification: the pre- and post-extraction workspace lists
  contain the same 99 named tests with SHA-256
  `94e0a9b5b01e4a66f8d9a88b24b816f7d51a5cadf302e903c2f3db99f19640a3`.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked` (99 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. The lock port, adapter, guard, acquisition, and release code moved as
one private responsibility; the parent preserves every public re-export and the
test-only seam. Directory/file ordering, identity verification, error/release
precedence, all 17 filesystem tests, and the exact 99-test inventory remain
unchanged. Every production filesystem module is now below 400 lines.
