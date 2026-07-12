---
id: story-split-status-application-status-adoption
kind: story
stage: review
tags: [refactor]
parent: feature-split-status-application
depends_on: [story-split-status-application-reconciliation]
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract status and adoption projection

## Brief

Move read-only status, native observation, and adoption projection into
`crates/cli/src/application/status.rs` after reconciliation extraction. Keep
document loading and scope/target resolution available to sibling modules while
preserving all status/adopt output and mutation boundaries.

## Current / target

Current `execute`, `execute_adopt`, `StatusDocuments`, `StatusScope`,
`StatusTargets`, `StatusProjection`, `NativeObservation`, adoption mapping,
document loading, scope resolution, and observation/path helpers occupy roughly
`application.rs:4290-6823`.

Target `status.rs` owns the two `pub(crate)` application methods and the status,
observation, projection, and adoption types/helpers under `pub(super)` or
private visibility. `load_documents` and `scope_request` are exposed only to
sibling application modules via `pub(super)`; `entrypoint.rs` is unchanged.

## Acceptance criteria

- First-use, missing/malformed owned documents (including redaction), target
  and scope errors, native observation failures, health ordering, and all
  global/project/all-scope output remain byte/schema compatible.
- Status remains read-only; adoption remains the only native-observation-to-
  inventory mutation path and retains revalidation/locking behavior.
- Unit, compiled-binary, isolated e2e tests, workspace fmt, tests, and clippy
  pass.

## Risk / rollback

Warning/resource ordering and observation helper visibility are easy to disturb.
If behavior moves, revert this extraction commit while retaining earlier module
steps; no state or native migration is required.

## Implementation Notes

- Moved status/adoption entrypoints, document loading, scope/target models,
  native observation, projection, and adoption error mapping into private
  `application/status.rs`.
- Exposed only the narrow `pub(super)` status support surface required by
  lifecycle, instruction, and reconciliation siblings; status remains
  read-only while adoption retains its existing lock/revalidation path.
- Verification: `cargo fmt --all`, `cargo check -p skilltap --offline`, and
  `cargo test -p skilltap --offline` passed (40 unit tests and 41 compiled-
  binary tests).
