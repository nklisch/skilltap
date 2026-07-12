---
id: gate-tests-sync-execution-integration
kind: story
stage: done
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

## Implementation Notes

Added `populated_plan_and_sync_apply_the_desired_inventory_resource`.
It seeds a desired plugin, removes only apply state to create an unapplied
inventory, verifies `plan` emits a scope/target-bound operation without
mutating inventory or state, then verifies `sync` journals the native apply
and an immediate repeat is idempotent.

Verification: the focused test passes. The full compiled-binary suite has 33
passing tests; the two unrelated scope/safe-update regressions remain blocked
on their parked production stories.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review with deep reconciliation correctness and
test-integrity lenses. The isolated regression proves populated inventory is
visible in `plan` without state mutation, `sync` executes through the lifecycle
adapter and journals state, and an immediate repeat is a no-change operation.
The focused test passes; the recorded full-suite failures are unrelated active
safe-update/scope stories.
