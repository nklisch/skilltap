---
id: feature-complete-status-application-extraction
kind: feature
stage: implementing
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Complete the StatusApplication support split

## Discovery finding

The completed `feature-split-status-application` extraction moved command
entrypoints into responsibility modules, but approximately 800 lines of
status-only support remain in `crates/cli/src/application.rs`. The residual
group includes first-use reporting, native observation tree/path projection,
native surface identity and health labels, update projection and revision
labels, and daemon-status projection (roughly lines 1448-1870 and
1748-2030). `status.rs` already owns the corresponding `StatusProjection` and
`NativeObservation` types and calls these helpers through `super::*`.

## Classification

Pure refactor: move the status/observation projection helpers next to the
status and adoption implementation. This completes an existing private module
boundary; no status semantics, observation limits, ordering, output, or
filesystem behavior may change.

## Target shape

Move the status-only helpers into `crates/cli/src/application/status.rs` (or a
private `application/status_support.rs` used only by that module):

- first-use harness reporting and daemon status projection;
- update candidate projection, update/revision labels, and Git revision-change
  comparison;
- observation tree/path labels and native surface resource projection;
- resource identity/kind/health/profile labels, capability counts, finding
  warning conversion, and observation IDs.

Keep genuinely cross-module helpers (`scope_label`, document/storage
projection, configured-binary parsing, and operation/lifecycle helpers) on the
parent support surface or extract them only when a dependency map proves the
new boundary is narrower. Do not create a generic utility module merely to
move unrelated functions.

## Guardrails

- Preserve read-only status and adoption mutation boundaries.
- Preserve first-use detection, native observation safety limits, malformed
  observation handling, warning/resource ordering, and update projection fields
  exactly.
- Preserve stable native resource IDs, FNV-1a labels, revision formatting,
  and all error/warning codes.
- Keep `status.rs`'s narrow `pub(super)` contract with lifecycle,
  reconciliation, and instruction modules; no public API or entrypoint wiring
  changes.
- Run status, adoption, observation, update, compiled-binary, formatting, and
  clippy checks after the mechanical move.

## Rejected candidates

Moving lifecycle and instruction helpers that are intentionally shared across
application child modules, or changing observation normalization and status
comparison behavior, would exceed a behavior-preserving support split and is
not part of this item.

## Design decisions

- Keep `crates/cli/src/application/status.rs` as the single private home for
  read-only status, observation, and update projection support. Do not create a
  generic utility module: the helpers all exist to assemble `StatusProjection`
  or `NativeObservation` output.
- Leave cross-command helpers on `application.rs`: `scope_label`, document and
  storage projection, `configured_binary`, `stable_hash`, lifecycle revision
  labels, and operation/lifecycle helpers are still consumed by lifecycle,
  instruction, reconciliation, or application code. They remain private
  parent support rather than being moved merely to reduce line count.
- Preserve the existing parent-module contract for reconciliation's first-use
  report with a narrow `pub(super)` re-export from `application.rs` when that
  function moves into `status.rs`. No entrypoint signature or public API
  changes are permitted.
- Treat each move as a mechanical extraction. Keep function bodies, imports,
  iteration order, bounded process/tree limits, warning/resource construction,
  stable IDs, and error labels byte-for-byte equivalent except for the module
  path and the minimum `pub(super)` visibility needed by the sibling module.
- Verification is layered: the status projection step runs status/update and
  compiled-binary tests; the observation-support step runs adoption,
  observation, status, and compiled-binary tests, then workspace formatting and
  clippy. Re-run the unchanged operation tests where they already establish
  idempotence.

## Refactor Overview

The first `StatusApplication` split moved command entrypoints but left two
status-only clusters in the parent module. The residual helpers make the
status child depend on an implicit parent implementation surface and force
read-only observation changes to be reviewed beside lifecycle and instruction
mutation code. The two-step extraction below completes the existing boundary
without changing any observable behavior.

## Refactor Steps

### Step 1: Extract status and update projections

**Priority**: High
**Risk**: Medium
**Source Lens**: code smell / missing abstraction (status projection mixed with lifecycle support)
**Files**: `crates/cli/src/application.rs`, `crates/cli/src/application/status.rs`, `crates/cli/src/application/reconciliation.rs` (visibility import only)
**Story**: `story-complete-status-application-extraction-status-projection`

**Current State**:

`application.rs` owns `first_use_harness_report`, `daemon_status_projection`,
`status_update_projection`, `update_projection_entry`, and their
update/revision/error-label helpers (approximately lines 1448–1735). The
status child calls these through `use super::*`; reconciliation also calls the
first-use report directly.

**Target State**:

```rust
// application/status.rs
pub(super) fn first_use_harness_report(...)
fn daemon_status_projection(...)
fn status_update_projection(...)
fn update_projection_entry(...)
fn update_decision_reason_label(...)
fn resolution_error_label(...)
struct UnavailableSourceRevisionResolver;
```

`application.rs` removes those definitions and re-exports only
`status::first_use_harness_report` as `pub(super)` so
`reconciliation.rs` keeps its existing call site. Existing lifecycle-facing
helpers (`revision_label`, `git_revision_changed`, and `harnesses_label`) stay
in the parent because lifecycle still calls them.

**Implementation Notes**:

- Copy the exact function bodies and imports into `status.rs`; use the parent
  imports through `use super::*` rather than introducing a second import graph.
- Preserve update candidate filtering by concrete scope and selected targets,
  update policy classification, warning ordering, and `available_updates`
  counting.
- Preserve first-use harness selection, disabled-versus-unreachable statuses,
  bounded detection limits, and warning contexts exactly.
- Keep `UnavailableSourceRevisionResolver` private to the status module and
  keep its unsupported-source error mapping unchanged.

**Acceptance Criteria**:

- [ ] The projection helpers and resolver have one definition, in
      `application/status.rs`; no status projection definition remains in the
      parent.
- [ ] Reconciliation still resolves its first-use call through the narrow
      parent re-export without changing its signature.
- [ ] Status/update unit tests and compiled-binary tests pass without changing
      assertions, output fields, warnings, or exit classes.
- [ ] `cargo fmt --all -- --check` passes for the extracted module.

**Risk**: Private visibility or an import accidentally selecting a different
resolver can alter status output or break reconciliation compilation.
**Rollback**: Revert this one extraction commit and restore the helper blocks
to `application.rs`; no state or native files are touched by the move.

---

### Step 2: Extract native observation and surface projection support

**Priority**: High
**Risk**: Medium
**Source Lens**: code smell / missing abstraction (native observation helpers outside the observation module)
**Files**: `crates/cli/src/application.rs`, `crates/cli/src/application/status.rs`
**Story**: `story-complete-status-application-extraction-observation-support`
**Depends On**: `story-complete-status-application-extraction-status-projection`

**Current State**:

`application.rs` owns the native observation tree/path helpers and status
surface projection helpers (approximately lines 1748–2022):
`observe_trees`, `instruction_surface_labels`, `path_exists`,
`child_path_exists`, `native_surface_resource`, `stable_resource_id`,
`native_surface_kind`, `observation_error`, `resource_identity`,
`resource_health`, `resource_kind`, `profile_authority`, `capability_count`,
`finding_warning`, and `observation_id`. `NativeObservation::run` in
`status.rs` calls them through the parent glob import.

**Target State**:

```rust
// application/status.rs
fn observe_trees(...)
fn instruction_surface_labels(...)
fn path_exists(...)
fn child_path_exists(...)
fn native_surface_resource(...)
fn stable_resource_id(...)
fn native_surface_kind(...)
fn observation_error(...)
fn resource_identity(...)
fn resource_health(...)
fn resource_kind(...)
fn profile_authority(...)
fn capability_count(...)
fn finding_warning(...)
fn observation_id(...)
```

`NativeObservation::run` and the status projection become self-contained in
`status.rs`. `stable_hash` remains on the parent support surface because
lifecycle and instruction operation IDs also use it; the moved
`stable_resource_id` calls that shared helper through `super::*`.

**Implementation Notes**:

- Preserve Codex/Claude observation-path selection, external-tree safety
  limits, symlink metadata checks, and surface-label ordering exactly.
- Preserve native surface resource identity, FNV-derived IDs, resource kind
  classification, profile authority, health labels, finding-to-warning
  conversion, and observation IDs exactly.
- Keep `configured_binary`, document/storage helpers, and scope helpers on the
  parent because other application children use them; do not introduce a
  generic utility module.
- Remove now-unused parent imports only after the full workspace compiles;
  avoid broad import churn that could conceal a behavior change.

**Acceptance Criteria**:

- [ ] All observation/surface helpers have one definition in `status.rs`, and
      `application.rs` contains no status-only observation projection blocks.
- [ ] Status, adoption, native observation, and compiled-binary tests pass with
      identical resource/warning order, stable IDs, limits, and malformed-state
      handling.
- [ ] `cargo fmt --all -- --check` and
      `cargo clippy --workspace --all-targets --offline -- -D warnings` pass.
- [ ] Repeating an unchanged status/adoption observation remains idempotent and
      does not write state or native files.

**Risk**: Moving a helper that is still consumed by lifecycle or instruction
code can create a visibility regression; changing path or hash helper context
can change resource identity.
**Rollback**: Revert only the observation-support extraction commit and restore
the helper blocks to the parent; retain the Step 1 projection move if it is
already verified.

## Implementation Order

1. `story-complete-status-application-extraction-status-projection`
2. `story-complete-status-application-extraction-observation-support`
