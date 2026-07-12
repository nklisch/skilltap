---
id: epic-real-harness-recovery-runtime-boundary-diagnostics
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-runtime-boundary
depends_on:
  - epic-real-harness-recovery-runtime-boundary-process-context
  - epic-real-harness-recovery-runtime-boundary-version-decoding
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Project actionable detection diagnostics

## Scope

Carry closed detection failure categories through harness list, first-use and
configured status, planning, and lifecycle capability lookup, with actionable
safe output and isolated compiled-binary coverage.

## Acceptance

- Every public detection surface agrees on reachability and failure kind.
- Invalid version output is distinguishable from an absent binary.
- JSON/plain output contains no native stdout, argv, environment values, or
  secrets and gives the next command appropriate to the failure.
- Isolated roots remain unchanged during every read-only scenario.

