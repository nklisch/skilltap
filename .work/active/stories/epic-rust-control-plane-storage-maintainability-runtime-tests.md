---
id: epic-rust-control-plane-storage-maintainability-runtime-tests
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

# Split Runtime Filesystem Tests by Contract

Mechanically split runtime filesystem tests into metadata/no-follow,
publication/copy recovery, ownership/link safety, and configuration-locking
modules. Preserve every test name, assertion, platform guard, and fault
scenario. Compare the test list and run the full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem/tests.rs` now owns only
  shared setup and textual child includes; contract-focused test bodies live in
  `tests/metadata.rs`, `tests/publication.rs`, `tests/ownership.rs`, and
  `tests/locking.rs`.
- Tests added: none. The pre/post list contains the same 20
  `runtime::filesystem::tests::*` identities; reconstructing the original file
  from the child files differs only by blank lines at file boundaries.
- Assertions and fault coverage: every test body, assertion, `cfg` guard,
  concurrency case, path-swap case, publication failure injection, residual
  check, and lock contention case is unchanged.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (150 tests), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. All twenty filesystem test identities, assertions, platform guards,
and fault scenarios are preserved across the four lexical contract files.
