---
id: feature-split-cli-daemon-commands
kind: feature
stage: review
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

## Refactor Overview

The daemon service-management boundary is a cohesive private CLI concern, but
it is currently embedded in `entrypoint.rs` between command dispatch and the
reconciliation/application commands. The boundary is large enough to obscure
dispatch ownership and forces daemon-only imports and test helpers into the
entrypoint module. A mechanical extraction into `daemon_commands.rs` improves
navigation and keeps service-file lifecycle code together without changing
the public `run_from` contract or any native behavior.

The extraction is intentionally staged by command. Each stage compiles while
the remaining commands still call their existing private functions; the final
stage moves shared service-manager helpers and completes the module boundary.

## Refactor Steps

### Step 1: Create the daemon command module and move enable/publication

**Priority**: High  
**Risk**: Medium  
**Source Lens**: code smell / missing abstraction  
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/daemon_commands.rs`, `crates/cli/src/entrypoint/tests.rs`
**Story**: `story-feature-split-cli-daemon-commands-module`

**Current State**:

`entrypoint.rs:288-451` owns `execute_system_daemon_enable`, the
`DaemonChangedFile` alias, and `publish_daemon_files`. The enable flow also
reaches back to the parent `run_service_manager` helper. Its rollback helper
is tested from `entrypoint/tests.rs` because it is private to that module.

**Target State**:

Declare a private `mod daemon_commands;` and move the enable flow, changed-file
alias, publication helper, and its focused rollback test into that module.
Expose only `pub(super) fn execute_system_daemon_enable` to the parent
dispatcher. During this incremental step the module may call the still-parent
`run_service_manager` and `ServiceManagerAction` through `super::`; those
helpers are removed in Step 3. Keep `repository_composition_error` and all
core/harness calls behaviorally identical.

**Implementation Notes**:

- Copy only the imports needed by the moved code into the new module; do not
  leave daemon-only imports in `entrypoint.rs` once their final consumers move.
- Update the dispatch arm to call `daemon_commands::execute_system_daemon_enable`
  while disable/status remain on their existing functions until later steps.
- Re-home `daemon_pair_publication_restores_earlier_service_files_on_later_failure`
  with the helper, preserving the test filesystem fixture and rollback order.

**Acceptance Criteria**:

- [ ] The new module builds and the enable dispatch produces byte-identical
  plain/JSON outcomes, warnings, resources, and exit classes.
- [ ] Publication rollback still restores prior bytes and removes newly-created
  files after a later write failure.
- [ ] Daemon enable idempotence, conflict, malformed, unreadable, and manager
  failure integration tests pass unchanged.
- [ ] `cargo test -p skilltap-cli --offline` passes after the move.

**Rollback**: Revert the extraction commit and restore the enable function and
publication test to `entrypoint.rs`; no service files or persisted state are
changed by this refactor.

---

### Step 2: Move daemon disable lifecycle

**Priority**: High  
**Risk**: Medium  
**Source Lens**: code smell / missing abstraction  
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/daemon_commands.rs`
**Story**: `story-feature-split-cli-daemon-commands-disable`

**Current State**:

`entrypoint.rs:453-543` owns `execute_system_daemon_disable`, including
platform-specific service names, ownership/malformed checks, manager disable,
and safe removal ordering. It directly uses the parent service-manager helper.

**Target State**:

Move the function into `daemon_commands.rs` as
`pub(super) fn execute_system_daemon_disable`, update only its dispatch call,
and preserve its access to the still-parent service-manager helper through
`super::`. Do not deduplicate inspection or alter the order of preflight,
manager invocation, and removal in this step; those are behavior-sensitive
and can be considered only after the complete extraction is reviewed.

**Implementation Notes**:

- Keep the existing `OutputArgs` signature and all `Outcome` construction
  unchanged.
- Keep unmanaged, malformed, unreadable, empty, manager-failure, and partial
  removal handling exactly as written.
- Leave status and its projection helpers in the parent until Step 3 so each
  intermediate commit remains buildable.

**Acceptance Criteria**:

- [ ] Disable dispatch resolves to the module function with identical output,
  exit code, and native calls.
- [ ] Empty disable remains a completed no-op; unmanaged or malformed owned
  definitions remain untouched; manager failure retains owned files.
- [ ] Existing daemon service-failure and idempotence tests pass without
  assertion changes.
- [ ] `cargo test -p skilltap-cli --offline` and workspace formatting pass.

**Rollback**: Revert the disable move and dispatch edit; Step 1's module and
enable behavior remain independently revertible.

---

### Step 3: Move daemon status, projections, and shared service-manager port

**Priority**: High  
**Risk**: Medium  
**Source Lens**: code smell / missing abstraction  
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/daemon_commands.rs`, `crates/cli/src/entrypoint/tests.rs`
**Story**: `story-feature-split-cli-daemon-commands-status`

**Current State**:

`entrypoint.rs:545-790` owns `execute_system_daemon_status`, result labels,
state-record field projection, recovery next actions, `ServiceManagerAction`,
and `run_service_manager`. The command dispatch and the three service flows
are interleaved with unrelated reconciliation and harness commands.

**Target State**:

Move status and all daemon-only projection/manager helpers into
`daemon_commands.rs`. The parent `run_from` dispatch retains stable command
signatures and calls `daemon_commands::execute_system_daemon_status`; no
daemon service-management implementation remains in `entrypoint.rs`.
Keep the module private and expose only the three `pub(super)` command
wrappers. Any test-only helper access uses a narrow `pub(super)` item or a
module-local test rather than making daemon internals public.

**Implementation Notes**:

- Move `daemon_result_label`, `daemon_record_fields`,
  `daemon_recovery_action`, `ServiceManagerAction`, and
  `run_service_manager` together with status so all service-manager calls
  share one private boundary.
- Preserve direct argument vectors, process limits, executable resolution,
  platform-specific manager commands, and all ownership/state validation.
- Remove now-unused daemon imports from `entrypoint.rs`; retain imports still
  used by reconciliation, harness, and composition functions.
- Keep plain and JSON field ordering/schema and `daemon_manager_unavailable`,
  `daemon_state_unavailable`, conflict, malformed, and unreadable diagnostics
  byte/structure compatible.

**Acceptance Criteria**:

- [ ] `entrypoint.rs` contains only daemon dispatch plus the separate
  reconciliation-backed `daemon run` command; no service lifecycle helpers or
  manager process code remain there.
- [ ] Daemon status reports disabled, installed, never-run, completed,
  pending, contended, and failed states exactly as before, including next
  actions and state fields.
- [ ] All daemon unit and compiled-binary tests pass; repeated enable/disable
  and status calls remain idempotent.
- [ ] `cargo fmt --all -- --check`, `cargo test --workspace --all-targets
  --offline`, `cargo clippy --workspace --all-targets --offline -- -D
  warnings`, and `git diff --check` pass.

**Rollback**: Revert this final move to restore the helper block in
`entrypoint.rs`; the command behavior and persisted artifacts are unchanged,
so rollback is a source-only operation.

## Implementation Order

1. `story-feature-split-cli-daemon-commands-module`
2. `story-feature-split-cli-daemon-commands-disable` (depends on Step 1)
3. `story-feature-split-cli-daemon-commands-status` (depends on Step 2)

## Implementation notes

- Execution capability: highest available local implementation context; this
  cross-platform private-boundary extraction preserved service lifecycle
  semantics and native manager calls.
- Review weight: standard (autopilot default).
- Implementation order: module/enable, disable lifecycle, then status and shared
  service-manager helpers.
- Verification: cargo check and cargo test -p skilltap --offline pass; the
  package suite reports 46 library tests, 43 compiled-binary tests, and 3
  plugin-package tests green.
- Discrepancies from design: none.
- Adjacent issues parked: none.
