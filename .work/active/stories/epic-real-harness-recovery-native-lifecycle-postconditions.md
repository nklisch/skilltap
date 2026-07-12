---
id: epic-real-harness-recovery-native-lifecycle-postconditions
kind: story
stage: implementing
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

# Verify lifecycle postconditions with actionable diagnostics

## Scope

Replace generic/indeterminate observation handling with typed native evidence
and require a fresh target/scope postcondition before lifecycle success is
journaled. This story owns blocker 10.

## Acceptance

- Native list command failures, malformed JSON, unsupported shapes, ambiguous
  scope, and unmet presence expectations have distinct stable diagnostics and
  actionable next steps.
- A successful install/add/update is recorded only when the resource is freshly
  present in the requested target/scope; removal requires freshly missing.
- Prior journal success plus indeterminate observation is attention-required,
  never a false no-op or automatic duplicate mutation.
- Failed post-observation does not publish successful state and preserves a safe
  retry path without attempting an unverified rollback.
- Disposable fake and real harness coverage proves success, each failure class,
  and immediate repeat idempotence without touching user configuration.
