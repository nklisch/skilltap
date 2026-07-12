---
id: story-complete-status-application-extraction-status-projection
kind: story
stage: implementing
tags: [refactor]
parent: feature-complete-status-application-extraction
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract status and update projection support

## Brief

Move read-only status projection helpers out of
`crates/cli/src/application.rs` and into the private
`crates/cli/src/application/status.rs` module. Preserve output ordering,
update policy classification, first-use detection, warning/error labels, and
the existing reconciliation call contract.

## Current / target

Current `application.rs` owns `first_use_harness_report`,
`daemon_status_projection`, `status_update_projection`,
`update_projection_entry`, the update/revision/error-label helpers, and
`UnavailableSourceRevisionResolver` (roughly lines 1448–1735). The status
child calls those helpers through the parent glob import, while
`reconciliation.rs` calls the first-use report directly.

Target `status.rs` owns all status-only projection helpers under private or
`pub(super)` visibility. `application.rs` removes their duplicate definitions
and re-exports only `status::first_use_harness_report` as `pub(super)` so the
reconciliation sibling keeps its existing call site. Lifecycle-facing
`revision_label`, `git_revision_changed`, and `harnesses_label` remain in the
parent because lifecycle still consumes them.

## Implementation notes

- Move function bodies and the resolver mechanically; use existing parent
  imports through `use super::*` and avoid behavior changes or broad import
  churn.
- Preserve concrete-scope/target filtering, update-mode and resource-intent
  classification, candidate/revision labels, warning ordering, and update
  counts exactly.
- Preserve first-use target resolution, disabled/unreachable statuses, bounded
  process limits, warning context, and JSON/plain output fields exactly.
- Keep the resolver's unsupported-source error mapping unchanged.

## Acceptance criteria

- [ ] Each moved helper has one definition in `application/status.rs`; no
      status projection implementation remains in the parent.
- [ ] Reconciliation compiles and uses the narrow first-use re-export without
      changing signatures or output.
- [ ] Status/update unit tests and compiled-binary tests pass with unchanged
      assertions and exit classes.
- [ ] `cargo fmt --all -- --check` passes.

## Risk / rollback

Private visibility or an import selecting a different resolver could alter
status output or break reconciliation. Revert this extraction commit to
restore the helper blocks to `application.rs`; no state or native files are
modified by the move.
