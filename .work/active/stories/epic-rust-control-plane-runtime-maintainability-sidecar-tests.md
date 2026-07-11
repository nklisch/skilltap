---
id: epic-rust-control-plane-runtime-maintainability-sidecar-tests
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Move Runtime Tests to Sidecars

Mechanically move the complete `filesystem` and `scope` private test modules to
`runtime/filesystem/tests.rs` and `runtime/scope/tests.rs`. Preserve module
names, private access, all source text, all 22 fully qualified test identities,
and the 93-test inventory. Run the full locked verification ladder.

## Implementation notes

- Files changed: `crates/core/src/runtime/filesystem.rs`,
  `crates/core/src/runtime/filesystem/tests.rs`,
  `crates/core/src/runtime/scope.rs`, and
  `crates/core/src/runtime/scope/tests.rs`.
- Tests added: none. The existing 12 filesystem tests and 10 scope tests moved
  mechanically into private sidecar modules.
- Test identity verification: `cargo test --locked --workspace -- --list`
  reported the same 93-test inventory before and after, and the sorted 22
  `runtime::{filesystem,scope}::tests::*` names were byte-identical.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (93 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. Both parent modules retain the same private `tests` module name; all
22 filesystem/scope fully qualified names and the complete 93-test inventory
match the baseline exactly. The change is mechanical and the focused listing
plus full locked verification remain green.
