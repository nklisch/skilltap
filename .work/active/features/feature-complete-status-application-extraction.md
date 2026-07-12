---
id: feature-complete-status-application-extraction
kind: feature
stage: drafting
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

