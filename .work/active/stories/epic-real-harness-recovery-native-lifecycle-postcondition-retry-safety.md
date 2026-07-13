---
id: epic-real-harness-recovery-native-lifecycle-postcondition-retry-safety
kind: story
stage: review
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on:
  - epic-real-harness-recovery-native-lifecycle-contracts
  - epic-real-harness-recovery-native-lifecycle-managed-project
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make failed postcondition retries observation-safe

## Finding

A native mutation can complete while its post-mutation list observation fails.
That attempt is correctly journaled as failed, but a later retry treats only
`Applied` and `NoChange` journals as requiring a fresh precondition. Once list
observation recovers, skilltap repeats the mutation even when the exact desired
presence already holds.

## Required fix

- Preserve enough typed journal evidence to distinguish a postcondition failure
  from a mutation command failure, or conservatively preobserve every retry with
  prior lifecycle evidence where duplicate mutation is possible.
- When recovered exact-scope observation already satisfies install/add/update
  presence or remove absence, return a completed no-op without spawning the
  native mutation.
- Keep indeterminate recovered observation attention-required and mutation-free.
- Do not reinterpret the failed journal as successful state or perform an
  unverified rollback.
- Add isolated compiled regressions for install and removal covering first
  postcondition failure, recovered observation, retry no-op, unchanged mutation
  count, and immediate repeat in plain and JSON output.

## Acceptance

- A successful native mutation followed by failed post-observation is never
  automatically repeated once recovered observation proves desired state.
- A recovered observation proving the opposite state permits exactly one safe
  retry mutation and requires a fresh successful postcondition.
- Indeterminate observation never authorizes a duplicate mutation.
- Failed post-observation still never publishes an `Applied` journal result.

## Implementation notes

- Execution capability: strongest inline implementation due native mutation and persisted-journal risk.
- Review weight: highest from the caller's autopilot instruction.
- Files changed: native lifecycle planning/execution, native-port locked revalidation, the native no-op operation constructor, and isolated compiled regressions.
- Tests added: recovered install/remove no-op, opposite-state single retry, indeterminate no-mutation, and under-lock observation race coverage.
- Discrepancies from design: native failed-attempt bindings are retained as recovery evidence while managed projection seeds remain success-only; this preserves the ownership safety introduced by the managed projection work.
- Adjacent issues parked: none.
