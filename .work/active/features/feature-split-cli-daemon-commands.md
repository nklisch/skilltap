---
id: feature-split-cli-daemon-commands
kind: feature
stage: drafting
tags: [refactor, infra]
parent: null
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract CLI daemon service commands

## Discovery finding

`crates/cli/src/entrypoint.rs` contains the complete daemon service-management
surface alongside command dispatch and application composition. The
`execute_system_daemon_enable`, `execute_system_daemon_disable`, and
`execute_system_daemon_status` functions (roughly lines 288-674) account for
more than 400 lines and share service-file naming, ownership validation,
platform path resolution, and outcome projection. The private
`publish_daemon_files`, `daemon_result_label`, `daemon_record_fields`, and
`daemon_recovery_action` helpers are part of the same boundary.

## Classification

Pure refactor: move the existing daemon command orchestration into a private
CLI module without changing service-manager calls, filesystem ordering,
ownership checks, rollback behavior, state-record projection, output strings,
or exit/result classification.

## Target shape

Create a private `crates/cli/src/daemon_commands.rs` (or equivalently named
private module) owning the three service commands and their helpers. Keep the
existing `entrypoint::run_from` dispatch and function signatures stable by
using narrow `pub(super)` wrappers or re-exports. The module may depend on the
existing `crate::daemon` service-definition helpers, core filesystem/runtime
ports, and outcome types; it must not introduce a second service-manager
implementation.

## Guardrails

- Preserve macOS launchd and Linux systemd-user file names and generated
  service contents exactly.
- Preserve owned-versus-unmanaged conflict handling and malformed owned-file
  refusal before writes or removal.
- Preserve atomic multi-file publication and rollback order, including the
  prior-bytes/removal distinction.
- Preserve manager enable/disable/status calls and their failure outcomes;
  no new fallback or retry behavior belongs in this extraction.
- Preserve daemon state-record fields, recovery next actions, warning codes,
  summaries, and plain/JSON output schemas.
- Verify the daemon enable, disable, status, manager-failure, conflict,
  malformed-file, and repeat-idempotence tests after the move.

## Rejected candidates

Changing service ownership validation, replacing the direct service-manager
port, or altering manager failure semantics would be behavior changes and are
outside this refactor.

