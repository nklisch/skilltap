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
