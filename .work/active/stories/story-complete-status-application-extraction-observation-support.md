---
id: story-complete-status-application-extraction-observation-support
kind: story
stage: done
tags: [refactor]
parent: feature-complete-status-application-extraction
depends_on: [story-complete-status-application-extraction-status-projection]
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract native observation support

## Brief

Move native observation tree/path and surface projection helpers out of
`crates/cli/src/application.rs` and into the private
`crates/cli/src/application/status.rs` module. Preserve observation limits,
malformed-state handling, stable resource identities, and read-only status and
adoption boundaries.

## Current / target

Current `application.rs` owns `observe_trees`,
`instruction_surface_labels`, `path_exists`, `child_path_exists`,
`native_surface_resource`, `stable_resource_id`, `native_surface_kind`,
`observation_error`, `resource_identity`, `resource_health`, `resource_kind`,
`profile_authority`, `capability_count`, `finding_warning`, and
`observation_id` (roughly lines 1748–2022). `NativeObservation::run` in
`status.rs` calls them through `use super::*`.

Target `status.rs` owns all of those helpers beside `NativeObservation` and
`StatusProjection`. `stable_hash` remains on the parent support surface
because lifecycle and instruction operation IDs also use it; the moved
`stable_resource_id` calls that shared helper through `super::*`. Shared
`configured_binary`, scope, document, and storage helpers remain in the
parent for sibling consumers.

## Implementation notes

- Move function bodies mechanically, retaining Codex/Claude path selection,
  external-tree limits, symlink metadata checks, and surface-label ordering.
- Keep FNV-derived native surface IDs, resource-kind and health mapping,
  profile authority, finding-to-warning conversion, and observation IDs
  byte/behavior compatible.
- Remove parent imports only after compilation proves they are unused; avoid
  unrelated import cleanup that could conceal a behavior change.
- Re-run unchanged status/adoption observation to prove no state or native
  writes occur during read-only paths.

## Acceptance criteria

- [ ] Each observation/surface helper has one definition in `status.rs`; no
      status-only observation block remains in `application.rs`.
- [ ] Status, adoption, native observation, and compiled-binary tests pass with
      identical resource/warning order, stable IDs, safety limits, and
      malformed-state handling.
- [ ] `cargo fmt --all -- --check` and
      `cargo clippy --workspace --all-targets --offline -- -D warnings` pass.
- [ ] Repeated unchanged status/adoption observation remains idempotent and
      does not persist state or modify native files.

## Risk / rollback

Moving a helper still consumed by lifecycle or instruction code can create a
visibility regression, while changing hash/helper context can change resource
identity. Revert this extraction commit and restore the helper blocks to the
parent; retain the verified status-projection step if already complete.

## Implementation

Implemented in commit 0d155dc. Native observation tree/path and surface
projection helpers now live beside NativeObservation in application/status.rs;
stable_hash, scope_label, and configured_binary remain shared parent support.
cargo test -p skilltap --offline passes, including status, adoption,
observation, and compiled-binary coverage. Formatting and clippy are pending
concurrent bootstrap changes outside this refactor.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate review, standard effective weight, Fast lane for this
low-risk behavior-preserving story. Fresh-context review confirmed the
observation and surface helpers have one definition in status.rs and retain
path selection, limits, malformed-state mapping, stable IDs, resource order,
and read-only boundaries. A detached worktree at 042e7ed passed `cargo fmt
--all -- --check`, the full workspace offline test suite, and `cargo clippy
--workspace --all-targets --offline -- -D warnings`; current bootstrap edits
were outside this refactor and were not included in that baseline.
