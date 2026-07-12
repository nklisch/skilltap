---
id: story-feature-extract-cli-bootstrap-boundary-daemon-policy
kind: story
stage: review
tags: [refactor, infra]
parent: feature-extract-cli-bootstrap-boundary
depends_on: [story-feature-extract-cli-bootstrap-boundary-publication]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract daemon binary update policy

## Brief

Move daemon-run binary update policy and service destination discovery out of
`entrypoint.rs` into `bootstrap_commands.rs`, then leave `entrypoint` with a
thin daemon-run composition wrapper. The daemon service lifecycle module stays
responsible for service files and manager operations.

## Current / target

`entrypoint.rs:1804-1962` currently owns
`execute_system_daemon_binary_policy`, `binary_policy_attention`, and
`daemon_binary_destination`. `execute_system_daemon_run` invokes that private
implementation before reconciliation, reaching into the foreground binary
publication helpers.

The target module owns the policy, attention projection, and destination
lookup, exposing one `pub(super)` policy function returning the existing
`Outcome`. `execute_system_daemon_run` remains in `entrypoint.rs` but only
calls the policy wrapper and then `execute_system_reconciliation`.

## Guardrails

- Preserve bootstrap mode `off`/`check`/`apply-safe`, persisted
  `allow_major`, lock path and destination selection, policy labels,
  `binary_changed`/`binary_pending` summaries, warnings, next actions, and
  result classes.
- Preserve launchd/systemd service roots and existing `crate::daemon` service
  executable extraction. Do not move service ownership or manager behavior.
- Preserve ordering: binary policy runs before application reconciliation and
  its same `Outcome` is merged by the lifecycle application.
- Keep compiled leaf, daemon, and application lifecycle tests unchanged; no
  new retries, output fields, or behavior are allowed.

## Acceptance criteria

- [ ] `entrypoint.rs` retains only a thin daemon-run wrapper; no daemon binary
      policy or destination helper remains there.
- [ ] Daemon `off`, check, apply-safe, missing/malformed service, lock
      contention, update, attention, and repeated-cycle outputs remain
      equivalent, including lifecycle merge behavior.
- [ ] Compiled binary, daemon, bootstrap, and application lifecycle tests pass
      with unchanged assertions.
- [ ] Full workspace format, offline tests, strict clippy, and diff checks pass.

## Risk / rollback

Moving the policy wrapper could alter update timing or service destination error
mapping. Restore the policy block and wrapper to `entrypoint.rs` on failure;
the source-only revert changes no daemon service files or binaries.

## Implementation notes

- Execution capability: highest available local implementation; daemon policy
  and destination discovery were moved mechanically with the publication
  boundary.
- Review weight: standard (autopilot default).
- Files changed: `crates/cli/src/bootstrap_commands.rs`,
  `crates/cli/src/entrypoint.rs`.
- Tests added: none; daemon policy, lifecycle merge, lock, and destination
  fixtures remain unchanged and continue to pass.
- Discrepancies from design: the policy extraction landed in the same source
  move as the preceding child stories so the private module had one complete,
  buildable owner; daemon lifecycle ownership and output contracts are
  unchanged.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap --offline` (59 unit/compiled/package tests passed)
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings`
- `git diff --check`
