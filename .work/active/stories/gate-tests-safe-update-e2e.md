---
id: gate-tests-safe-update-e2e
kind: story
stage: review
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Cover safe-update status and daemon state transitions

## Priority

High

## Spec reference

Items `epic-safe-update-automation-resolution-orchestration`,
`epic-safe-update-automation-policy-status`,
`epic-safe-update-automation-service-run`, and
`epic-safe-update-automation-diagnostics`.

## Gap type

Missing revision-availability, policy, safe-cycle, failure-mode, repeat, and
daemon-record integration coverage.

## Suggested test

Use Git/native fixtures to exercise changed revisions, update modes, pinned and
drifted resources, partial/lock/source failures, repeat no-op behavior, and
`state.json` available-revision and daemon-run records.

## Test location (suggested)

`crates/cli/tests/compiled_binary.rs` and application integration tests.

## Implementation Notes

Added `safe_update_cycle_reports_changed_git_revision_and_records_daemon_result`.
It creates an isolated Git-backed complete skill tree, verifies daemon no-op
state journaling, advances the upstream commit, checks status update evidence,
then verifies the daemon applies the new tree and records its run.

The daemon result-classification and Git-backed update-path fixes landed in
`24c3ffc` and `314eed5`. The fixture now passes the explicit `--name
daemon-skill` so the managed destination matches the frontmatter identity and
the daemon's inventory selector.

Verification: the focused safe-update test passes against the fixed
implementation, including no-op and changed-revision daemon records.

Added `safe_update_policy_pins_drift_and_source_failures_remain_visible`.
It covers check/off/apply-safe policy observations, a pinned resource blocked
from automatic replacement, local destination drift preservation, unavailable
Git source handling, and daemon-run state retention.

Verification: both safe-update focused tests pass, including the existing
Git revision transition regression.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: the required safe-update policy and failure matrix is not covered
(this item)
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context substrate review with correctness, tests,
update-safety, and daemon-state lenses. The Git fixture correctly proves one
clean no-op, an available revision, a changed revision, and a persisted daemon
record. The acceptance criteria additionally require policy modes, pinned and
drifted resources, partial/lock/source failures, repeat no-op behavior, and
available-revision plus daemon-run state assertions across those outcomes.
Those cases are absent; extend the isolated matrix without weakening the
existing regression.

## Follow-up Resolution

Added `safe_update_policy_pins_drift_and_source_failures_remain_visible`.
It covers check/off/apply-safe policy observations, a pinned resource blocked
from automatic replacement, local destination drift preservation, unavailable
Git source handling, and daemon-run state retention. Both safe-update focused
tests pass, including the Git revision transition regression.

Added Linux-isolated lock contention coverage with a real `flock` holder:
daemon update reports `configuration_locked`, records pending work, and
recovers to apply the revision after the lock releases. Drift and unavailable
source assertions now also require pending-operation accounting and the
persisted daemon `pending` result.
