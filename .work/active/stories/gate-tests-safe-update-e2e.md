---
id: gate-tests-safe-update-e2e
kind: story
stage: implementing
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

The initial no-op assertion currently exposes a production defect: the daemon
returns `attention_required` despite `changed=false` and safe work completing,
because its aggregate outcome retains the document-load attention state.
Parked as `idea-daemon-noop-result-class`; do not weaken or skip this test. The
story remains blocked on that fix and review of the resulting regression.

## Blocker

The regression cannot pass until daemon aggregate results classify a successful
no-op as completed. See parked item `idea-daemon-noop-result-class`.
