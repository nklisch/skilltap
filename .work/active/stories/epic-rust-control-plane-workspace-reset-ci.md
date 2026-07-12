---
id: epic-rust-control-plane-workspace-reset-ci
kind: story
stage: done
tags: [infra]
parent: epic-rust-control-plane-workspace-reset
depends_on: [epic-rust-control-plane-workspace-reset-workspace]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Establish Rust CI and Binary Verification

## Scope

Implement Unit 2 from the parent feature: replace product CI with Rust format,
clippy, test, release-build, and compiled-binary smoke gates. Rewrite the smoke
script for the Rust artifact and keep it extensible for later CLI commands.

## Acceptance criteria

- [x] Every CI command passes locally.
- [x] CI has no Bun/npm product dependency.
- [x] Binary verification checks `--version` and `--help` and fails clearly for
  a missing or broken executable.

## Implementation notes

- Files changed: `.github/workflows/ci.yml`, `scripts/verify-binary.sh`.
- Tests added: no persistent test files; locally executed every CI command and
  negative smoke cases using a missing path, `/bin/true`, and `/bin/false`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Story verified by implement and integrated Rust/shell checks;
fast-lane advance.
