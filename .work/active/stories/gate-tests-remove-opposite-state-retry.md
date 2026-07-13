---
id: gate-tests-remove-opposite-state-retry
kind: story
stage: review
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Cover remove retry when recovered observation remains present

## Priority
Medium

## Spec reference
`epic-real-harness-recovery-native-lifecycle-postcondition-retry-safety`:
opposite recovered state permits exactly one safe retry with fresh
postcondition and an immediate no-op repeat.

## Required test

Drive a remove through an indeterminate post-observation while leaving the
native resource present. After observation recovers, require exactly one retry,
a fresh missing postcondition, and an immediate no-op repeat.

## Implementation

- Added a native fixture mode where remove returns success, post-observation is
  indeterminate, and the resource remains present.
- Verified recovered presence permits exactly one retry, the fresh
  postcondition proves missing, and the immediate repeat performs no mutation.

## Verification

- `cargo test -p skilltap --test native_postconditions failed_remove_postobservation_retries_once_when_resource_is_still_present -- --exact --nocapture`
