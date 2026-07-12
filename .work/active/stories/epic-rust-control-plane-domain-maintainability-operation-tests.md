---
id: epic-rust-control-plane-domain-maintainability-operation-tests
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

# Externalize Operation Domain Tests

Mechanically move the inline `operation.rs` test module to
`operation/tests.rs`. Preserve every test, fixture, assertion, name, and private
item access. Production code, public API, serde forms, and behavior must not
change. Verify the same 56 core tests plus locked workspace checks.

## Implementation notes

- Files changed: `crates/core/src/domain/operation.rs` and
  `crates/core/src/domain/operation/tests.rs`.
- Mechanically replaced the inline `#[cfg(test)] mod tests { ... }` suffix with
  `#[cfg(test)] mod tests;` and moved its complete body into the child module.
- Preserved all 18 operation test names, fixtures, assertions, and private-item
  access; rustfmt changed layout only after removing the wrapper indentation.
- Production code, public APIs, serde forms, errors, and behavior are unchanged.
- Locked format, check, clippy with warnings denied, and workspace tests pass;
  the core inventory remains 56 tests.
- Adjacent issues parked: none.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Production prefix is unchanged, all 18 operation tests are
preserved, and the locked 56-test workspace suite passes; fast-lane advance.
