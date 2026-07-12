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
