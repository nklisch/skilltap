---
id: gate-tests-execution-acknowledgment-exact-match
kind: story
stage: drafting
tags: [testing]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: tests
created: 2026-04-02
updated: 2026-07-15
---

# Cover exact execution-acknowledgment validation

## Priority
Medium

## Value evidence
Item: `epic-expanded-harness-support-declaration-managed`

The declaration-managed contract requires changed, missing, or extra acknowledgment requirements to be rejected. `ExecutionAcknowledgments::new` implements exact operation-id and requirement matching, but no production or test caller currently exercises it.

## Gap type
complex-unit

## Suggested test

Build a plan with one partial operation. Verify that the exact `(operation id, requirement)` pair is accepted and that an unknown id, a non-partial operation id, and a changed requirement each return the documented `GraphError` variant.

## Test location
`crates/core/src/executor.rs`
