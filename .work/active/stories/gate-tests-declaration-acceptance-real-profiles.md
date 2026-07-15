---
id: gate-tests-declaration-acceptance-real-profiles
kind: story
stage: done
tags: [testing]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: tests
created: 2026-04-02
updated: 2026-07-15
---

# Exercise declaration acceptance against real unverified profiles

## Priority
Medium

## Value evidence
Item: `epic-expanded-harness-support-declaration-managed`

For most scenarios, `exercise_declaration_managed_acceptance` delegates to `exercise_fake_managed_acceptance`, which uses a Supported profile. That does not exercise the declaration-specific acknowledgment gate, effective-unverified status, or daemon exclusion for Kimi, Vibe, Kilo, Junie, and Amp.

## Gap type
e2e-seam / important-interface

## Suggested test

Exercise the real declaration-managed profiles through a shared acceptance path that proves: install without `--yes` performs no target write and reports `partial_operation_requires_acknowledgment`; install with `--yes` applies the declaration while status remains effective-unverified; and daemon execution leaves the declaration pending without writing the target.

## Test location
`crates/cli/src/application/tests.rs` and, where process isolation matters, `crates/cli/tests/compiled_binary.rs`

## Verification

Exercised real unverified declaration-managed profiles through unacknowledged zero-write, acknowledged effective-unverified, and daemon no-write paths.

- Focused core, application, and compiled-binary tests pass.
- `cargo test --workspace --all-targets`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- Independent standard review: no material findings.
- `cargo fmt --all -- --check` and `git diff --check`: clean.
