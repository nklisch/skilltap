---
id: epic-rust-control-plane-runtime-maintainability-temp-roots
kind: story
stage: done
tags: [refactor, testing]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Share Test Temporary Roots

Add a generic unique temporary-root owner to `skilltap-test-support` and consume
it from command, filesystem, and scope tests through a core dev-dependency.
Keep module-specific fixture behavior local, preserve paths/test identities and
assertions, use best-effort cleanup without hiding an active panic, and run the
full locked verification ladder.

## Implementation notes

- Added `skilltap_test_support::TempRoot`, which creates collision-resistant
  process/sequence-named directories and removes them on a best-effort basis in
  `Drop` without panicking during unwinding.
- Registered `skilltap-test-support` as a workspace dependency used by
  `skilltap-core` only under `[dev-dependencies]`. Test support has no dependency
  on core or its domain types.
- Command, filesystem, and scope tests retain their local fixture wrappers,
  path prefixes, setup logic, test names, and assertions; only temporary-root
  creation and ownership moved to the shared helper.
- Tests added: `tests::roots_are_unique_created_and_removed_on_drop` in
  `skilltap-test-support`. All 93 pre-existing test identities remained exact;
  the workspace inventory is now 94 tests.
- Files changed: `Cargo.toml`, `Cargo.lock`, `crates/core/Cargo.toml`,
  `crates/test-support/src/lib.rs`,
  `crates/core/src/runtime/command.rs`,
  `crates/core/src/runtime/filesystem/tests.rs`, and
  `crates/core/src/runtime/scope/tests.rs`.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (94 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. `TempRoot` owns only generic unique-root creation/path access and
best-effort cleanup; test-support remains independent of core. Command,
filesystem, and scope keep their local fixture semantics. All 93 prior test
identities remain exact and the new lifecycle test brings the green workspace
inventory to 94.
