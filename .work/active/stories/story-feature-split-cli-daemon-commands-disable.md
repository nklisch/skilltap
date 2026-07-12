---
id: story-feature-split-cli-daemon-commands-disable
kind: story
stage: done
tags: [refactor, infra]
parent: feature-split-cli-daemon-commands
depends_on:
  - story-feature-split-cli-daemon-commands-module
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract daemon disable lifecycle

## Brief

Move `execute_system_daemon_disable` into the private
`daemon_commands.rs` module and update its dispatch call. Preserve the existing
preflight ownership checks, manager disable invocation, safe removal order,
outcomes, warnings, and no-op behavior. The function may temporarily call the
parent `run_service_manager` helper until the status child completes the
boundary.

## Current / target

Current code is `entrypoint.rs:453-543`, where disable constructs platform
service paths, refuses unmanaged/malformed/unreadable definitions, disables
the user manager, and removes only owned files.

Target is `daemon_commands::execute_system_daemon_disable` with the same
`&OutputArgs` signature and the parent dispatcher calling that wrapper. No
inspection deduplication or semantic cleanup is part of this move.

## Acceptance criteria

- Empty disable remains a completed, unchanged no-op.
- Unmanaged, malformed, unreadable, and manager-failure paths preserve files,
  warning codes, result classes, and exit codes exactly.
- Existing service-failure and repeated enable/disable integration tests pass
  without assertion changes.
- `cargo test -p skilltap-cli --offline` and workspace formatting pass.

## Risk / rollback

The extraction can accidentally change module privacy or evaluation order.
Keep the function body mechanical and use the existing parent manager helper.
Revert the move and dispatch edit to roll back without touching service files or
state.

## Implementation notes

- Execution capability: highest available local implementation context; the
  extraction crosses platform-specific service ownership and manager handling.
- Review weight: standard (autopilot default).
- Files changed: crates/cli/src/daemon_commands.rs and
  crates/cli/src/entrypoint.rs.
- Tests added: none; existing service-failure and idempotence coverage remains
  unchanged.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: Acceptance text names the legacy package selector skilltap-cli;
the actual package is skilltap.

**Notes**: Substrate review, fresh-context standard pass. Mechanical private
module extraction preserves disable preflight, ownership checks, manager
failure handling, removal ordering, and output behavior. cargo test -p
skilltap --offline, cargo fmt --all -- --check, workspace clippy with
warnings denied, and git diff --check pass.
