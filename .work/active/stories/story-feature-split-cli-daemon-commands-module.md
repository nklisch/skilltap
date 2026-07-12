---
id: story-feature-split-cli-daemon-commands-module
kind: story
stage: done
tags: [refactor, infra]
parent: feature-split-cli-daemon-commands
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract daemon enable and publication boundary

## Brief

Create the private `crates/cli/src/daemon_commands.rs` module and move the
daemon enable command, its changed-file publication/rollback helper, and the
focused publication test out of `entrypoint.rs`. Keep the existing
`run_service_manager` parent helper as a temporary `super::` dependency; the
final helper move is the status child story.

## Current / target

Current code is `entrypoint.rs:288-451` (`execute_system_daemon_enable`,
`DaemonChangedFile`, and `publish_daemon_files`) with the rollback test in
`entrypoint/tests.rs:268-301`.

Target is a private `daemon_commands` module exposing only
`pub(super) fn execute_system_daemon_enable` to the dispatcher. The moved
function preserves config loading/default interval, service-definition
rendering, ownership/malformed checks, atomic publication, manager activation,
and all output projection exactly.

## Acceptance criteria

- The enable dispatch calls the module wrapper and no duplicate enable/helper
  definitions remain in `entrypoint.rs`.
- Publication rollback restores previous bytes and removes a newly-created
  file after a later write fails; the focused test is module-local.
- Daemon enable idempotence, conflict, malformed, unreadable, and manager
  failure compiled-binary tests pass unchanged.
- `cargo test -p skilltap-cli --offline` and `cargo fmt --all -- --check`
  pass.

## Risk / rollback

The main risk is privacy/import wiring while the manager helper remains in the
parent. Keep all service writes and output construction mechanical. Revert the
extraction commit to restore the original function/test locations; no runtime
state or service files are changed by this refactor.

## Implementation notes

- Execution capability: highest available local implementation context; daemon
  service publication is a cross-platform lifecycle boundary.
- Review weight: standard (autopilot default).
- Files changed: crates/cli/src/daemon_commands.rs,
  crates/cli/src/entrypoint.rs, and crates/cli/src/entrypoint/tests.rs.
- Tests added: publication rollback test moved with its private helper into the
  daemon command module; assertions are unchanged.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: Acceptance text names the legacy package selector skilltap-cli;
the actual package is skilltap.

**Notes**: Substrate review, fresh-context standard pass. Mechanical private
module extraction preserves enable dispatch, publication ordering, rollback,
and output behavior. cargo test -p skilltap --offline, cargo fmt --all
-- --check, workspace clippy with warnings denied, and git diff --check pass.
