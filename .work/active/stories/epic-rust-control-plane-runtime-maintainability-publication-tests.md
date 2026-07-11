---
id: epic-rust-control-plane-runtime-maintainability-publication-tests
kind: story
stage: done
tags: [refactor, testing]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Split Publication Recovery Scenarios

Replace the single six-scenario publication failure test with focused tests for
clean rollback, pre-publication temp residue, destination-only residue,
temporary-only residue, both residuals, and sync-only uncertainty. Preserve the
existing injected seam and assertions, add no abstraction without three uses,
and run the full locked verification ladder.

## Implementation notes

- Replaced
  `backup_failures_report_exact_residual_paths_and_independent_sync_state` with
  six focused tests covering clean rollback, pre-publication temporary residue,
  destination-only residue, temporary-only residue, both residuals with safe
  rendering, and sync-only uncertainty.
- Preserved `InjectedPublication`, `partial_residuals`, `residual_roles`,
  `assert_residual_paths_exist`, and the complete original assertion multiset.
  Added only `publication_source`, a shared setup helper used by all six cases.
- Test identity verification: the expected one-to-six replacement increased the
  workspace inventory from 94 to 99 tests; every unrelated identity remained
  unchanged.
- Files changed: `crates/core/src/runtime/filesystem/tests.rs`.
- Production changes: none.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (99 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. The broad test became six independently diagnosable scenarios while
retaining every injected failure and assertion; the only helper is shared by
all six cases. No production code changed, unrelated identities are unchanged,
and the expected inventory rises from 94 to 99 with the full locked ladder
green.
