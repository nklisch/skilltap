---
id: epic-real-harness-recovery-runtime-boundary-diagnostics-completion
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

# Complete typed diagnostics across lifecycle surfaces

## Finding

Harness list and status use the closed detection diagnostic mapper, but native
lifecycle capability lookup discards `DetectionError` and emits a generic
profile warning. Two compiled tests also retain assertions for the removed
generic warning code, leaving the exact implementation baseline red.

## Required fix

- Return the typed detection failure from lifecycle capability lookup and
  project it through the same authoritative warning/next-action mapper used by
  harness list and status.
- Keep an unsupported exact profile or unverified capability distinct from a
  failed version detection.
- Update the two stale sibling-observation/adoption compiled tests to assert
  the exact typed category and safe next action.
- Add lifecycle compiled coverage for absent executable, invalid version,
  nonzero version command, and bounded failure without raw stdout, argv,
  environment, or secret leakage.
- Run the full all-target workspace suite, formatting, and all-feature clippy.

## Acceptance

- Harness list, status/adopt/plan, and lifecycle commands agree on the same
  detection category and next action.
- Unsupported profile/capability remains a separate post-detection outcome.
- The full workspace suite is green.
