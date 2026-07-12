---
id: story-feature-split-cli-daemon-commands-status
kind: story
stage: implementing
tags: [refactor, infra]
parent: feature-split-cli-daemon-commands
depends_on:
  - story-feature-split-cli-daemon-commands-disable
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Complete daemon command module with status and service-manager helpers

## Brief

Move `execute_system_daemon_status`, daemon result/projection helpers,
`ServiceManagerAction`, and `run_service_manager` into
`crates/cli/src/daemon_commands.rs`. Complete the private module boundary so
`entrypoint.rs` retains only daemon dispatch and the separate reconciliation
backed `daemon run` command.

## Current / target

Current code is `entrypoint.rs:545-790`: status loads state and service files,
projects daemon result fields and recovery actions, and the shared manager
helper executes bounded launchd/systemd-user vectors.

Target is a private module exposing only the three daemon service command
wrappers (`enable`, `disable`, `status`). Service manager calls, result labels,
state projection, and next-action mapping are module-private. Dispatch
signatures, output schemas, process limits, argument vectors, ownership checks,
and all evaluation ordering remain unchanged.

## Acceptance criteria

- `entrypoint.rs` has no daemon service lifecycle helper, service-manager
  process code, or daemon-only projection function after the move.
- Disabled, installed, never-run, completed, pending, contended, failed,
  conflict, malformed, unreadable, and malformed-state status outputs remain
  byte/structure compatible, including next actions and exit classes.
- All daemon unit and compiled-binary tests pass, including repeat idempotence
  and service-manager failure paths.
- `cargo fmt --all -- --check`, `cargo test --workspace --all-targets
  --offline`, `cargo clippy --workspace --all-targets --offline -- -D
  warnings`, and `git diff --check` pass.

## Risk / rollback

This is a private-module move, but status has many output branches and the
manager helper is shared by all three commands. Preserve direct argument
vectors and result construction exactly; use a mechanical move with no
deduplication. Revert the final extraction commit to restore the helper block
and dispatch calls in `entrypoint.rs`.
