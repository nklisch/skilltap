---
id: epic-rust-control-plane-runtime-maintainability-publication-module
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-unix-identity]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Publication State Machine

Move the private recoverable-copy staging, no-clobber publication, cleanup,
rollback, and residual construction machinery to
`runtime/filesystem/publication.rs`. Preserve the public `FileSystem` method,
all injected test seams, exact error precedence/state, and exports. Run the
complete recovery matrix and full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem.rs` and new private
  `crates/core/src/runtime/filesystem/publication.rs`.
- Extraction: moved the private publication trait and system implementation,
  recoverable-copy staging, no-clobber publication, identity-safe cleanup,
  rollback, and residual construction into the publication sidecar.
- Shared primitives: temporary allocation and parent-directory sync remain in
  `filesystem.rs` because atomic writes use them too; the publication module
  accesses them only through private module visibility.
- Test seam preservation: the injected `Publication` implementation,
  `copy_recoverable_with`, and `SystemPublication` remain available to the
  existing sidecar tests through test-only/private parent imports.
- Behavior preservation: the public `FileSystem::copy_recoverable` method,
  staging and rollback order, native operations, error precedence, residual
  roles, directory-sync states, and public exports are unchanged.
- Tests added: none; the complete 17-test filesystem/recovery matrix passes.
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

Approved. Only the private publication state machine and its narrow injected
test seam moved; shared atomic-write helpers and locking remain in the parent as
designed. Error precedence, residual ordering/sync state, public API, all 17
filesystem tests, and the exact 99-test workspace inventory are unchanged.
