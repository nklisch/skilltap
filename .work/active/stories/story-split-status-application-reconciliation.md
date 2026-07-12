---
id: story-split-status-application-reconciliation
kind: story
stage: done
tags: [refactor]
parent: feature-split-status-application
depends_on: [story-split-status-application-lifecycle, story-split-status-application-instructions]
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract plan and sync reconciliation

## Brief

Move plan/sync entrypoints and reconciliation candidate projection into
`crates/cli/src/application/reconciliation.rs` once lifecycle and instruction
adapters have stable private module locations. Keep reconciliation an
orchestrator over those adapters; do not duplicate native execution or mutate
from `plan`.

## Current / target

Current `execute_plan`, `execute_sync`, and `execute_reconciliation` are split
between `application.rs:633-651` and `application.rs:3781-4290`; selector
matching, source/name projection, scope conversion, outcome merging, and result
ranking are top-level helpers.

Target `reconciliation.rs` owns an `impl StatusApplication<'_>` with the same
`execute_plan`, `execute_sync`, and private `execute_reconciliation` signatures,
plus reconciliation-only helpers. It calls lifecycle and instruction methods
from sibling modules and keeps the existing `acknowledged` boolean, selectors,
target/scope filtering, and observation behavior.

## Acceptance criteria

- Populated and empty inventory plans/syncs route supported resources through
  the same existing adapters and preserve operation counts/statuses.
- Include/exclude selectors, project/global/all-scopes, target selection,
  partial `--yes`, observation failures, mutation-free plan, and repeated sync
  idempotence remain unchanged.
- Release reconciliation tests and workspace fmt, tests, and clippy pass with
  no output diff.

## Risk / rollback

Outcome merge or helper import changes may compile while changing result
classification. Revert this commit and restore orchestration to the parent;
lifecycle and instruction modules remain independently usable.

## Implementation Notes

- Moved plan/sync entrypoints and reconciliation orchestration into private
  `application/reconciliation.rs`, including selector/source projection and
  outcome aggregation helpers.
- Plan and sync continue to route through lifecycle/instruction adapters with
  unchanged scope/target filtering, acknowledgment, observation, and result
  classification behavior.
- Verification: `cargo fmt --all`, `cargo check -p skilltap --offline`, and
  `cargo test -p skilltap --offline` passed (40 unit tests and 41 compiled-
  binary tests).

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard same-harness fresh-context review. Reconciliation remains
an orchestrator over existing adapters; plan stays mutation-free and sync
preserves candidate ordering, selectors, target/scope boundaries, outcomes,
and generic acknowledgment. Workspace fmt, tests, clippy, and diff checks are
green.
