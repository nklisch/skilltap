---
id: epic-rust-control-plane-domain-maintainability-resource-tests
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-domain-maintainability
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-12
---

# Externalize Resource Domain Tests

Mechanically move the inline `resource.rs` test module to
`resource/tests.rs`. Preserve every test, fixture, assertion, name, and private
item access. Production code, public API, serde forms, and behavior must not
change. Verify the same 56 core tests plus locked workspace checks.

## Implementation notes

- Files changed: `crates/core/src/domain/resource.rs`,
  `crates/core/src/domain/resource/tests.rs`, and this story.
- Mechanically moved the complete inline test-module body into the child module
  and replaced it with `#[cfg(test)] mod tests;`.
- Preserved all 16 test names, 10 fixture helpers, assertions, and private parent
  access; no production code, API, serde, error, or behavior changed.
- Verification: `cargo fmt --all -- --check`, `cargo check --workspace --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, and
  `cargo test --workspace --locked` pass with the unchanged 56 core tests.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Production prefix is unchanged, all 16 resource tests and 10 helpers
are preserved, and the locked 56-test workspace suite passes; fast-lane advance.
