---
id: gate-tests-sync-execution-integration
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

# Cover populated plan and sync execution

## Priority

High

## Spec reference

Items `epic-reconciliation-execution-cli`; synchronization contract in
`docs/SPEC.md`.

## Gap type

Missing end-to-end lifecycle planning, execution, journaling, and repeat-no-op
coverage for populated desired state.

## Suggested test

Create a desired skill/plugin and native drift in an isolated environment;
assert `plan` produces scope/target-bound operations, `sync` mutates through
the lock/journal, and an immediate second sync is no-change with no mutator
calls.

## Test location (suggested)

`crates/cli/tests/compiled_binary.rs`
