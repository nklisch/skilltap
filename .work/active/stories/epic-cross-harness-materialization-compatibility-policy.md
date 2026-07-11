---
id: epic-cross-harness-materialization-compatibility-policy
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-compatibility
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Component Compatibility Policy

Implement the single capability-rule registry and per-component analyzer in
`crates/core/src/compatibility.rs`.

Acceptance criteria:

- Supported, unsupported, unverified, collision, and unknown-kind outcomes are
  target-bound and validated through `CompatibilityResult`.
- Requiredness controls blocked versus partial classification.
- Every non-faithful result has exact evidence and consequence data.
