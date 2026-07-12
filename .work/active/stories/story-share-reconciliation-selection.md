---
id: story-share-reconciliation-selection
kind: story
stage: done
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Share reconciliation selection iteration

## Value

Plan and sync currently duplicate resource/target filtering and tuple
construction. A shared iterator will prevent selection-order or scope/target
drift while preserving the existing BTreeSet ordering and command behavior.

## Scope

Extract a private helper around the plan loop near `application.rs:3862-3922`
and sync loop near `application.rs:3952-4020`. Keep selection, scope, target,
and resource-kind semantics identical.

## Acceptance

- Plan and sync select the same resources and targets in the same order.
- Existing plan/sync compiled regressions remain green.
- No public signatures or output schema changes.

## Implementation Notes

- Added `reconciliation_selections`, a shared ordered projection of desired
  resources, selected scopes/targets, and include/exclude selectors.
- Both plan and sync now consume the same `(resource, target)` sequence; the
  desired resource count remains deduplicated independently of target count.
- Verification: `cargo fmt --all` and `cargo test -p skilltap --offline`
  passed (40 unit tests and 41 compiled-binary tests).

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard same-harness fresh-context review. Plan and sync consume
the same BTreeSet-backed resource/target order, preserve selector and scope
filters, and keep resource counts deduplicated; workspace fmt, tests, clippy,
and diff checks are green.
