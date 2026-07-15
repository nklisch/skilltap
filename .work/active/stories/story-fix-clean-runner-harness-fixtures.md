---
id: story-fix-clean-runner-harness-fixtures
kind: story
stage: review
tags: [bug, testing]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: tests
created: 2026-07-15
updated: 2026-07-15
---

# Isolate verified harness identities in clean-runner tests

## Symptom

Both branch and tag CI fail `managed_projection_profiles_pass_the_shared_acceptance_matrix_repeatedly` because Codex resolves as missing. The release build's compiled-binary contract also fails standalone skill lifecycle tests because Codex and Claude profiles cannot be verified on clean runners.

## Root cause

Application tests call `enable_codex_only` without configuring an isolated executable, and compiled-binary tests write `ENABLED_CONFIG` with bare `codex` and `claude` names while inheriting the host `PATH`. The tests pass on a developer machine with those binaries installed but fail deterministically on clean Linux and macOS runners.

## Fix approach

Materialize exact-profile Codex and Claude fake harnesses inside each isolated test root, configure their absolute paths, and retain the fixtures for the test lifetime. Replace ambient `ENABLED_CONFIG` use with an explicit helper that owns those fakes. Do not alter production detection or mutation authority.

## Regression test

The existing managed-projection acceptance matrix and compiled standalone-skill lifecycle tests are the regression guards. Reproduce them with a scrubbed executable search path before the fix, then verify the focused tests, full workspace suite, and release compiled-binary contract on clean paths after the fix.

## Implementation notes

- **Execution capability:** inline focused repair; the defect is confined to test fixture composition and requires no production or public-interface change.
- **Files changed:** `crates/cli/src/application/tests.rs` and `crates/cli/tests/compiled_binary.rs`.
- **Regression evidence:** the application acceptance matrix failed with `native_executable_not_found` under `PATH=/usr/bin:/bin` before the fix and passes afterward. All 79 compiled-binary tests pass under the same scrubbed path.
- **Full confirmation:** `cargo test --locked --workspace --all-targets`, strict workspace Clippy, formatting, and diff checks pass.
- **Original symptom:** clean Linux and macOS CI no longer require developer-installed Codex or Claude binaries.
- **Adjacent work:** none bundled or parked.
