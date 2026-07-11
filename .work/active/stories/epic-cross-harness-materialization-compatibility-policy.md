---
id: epic-cross-harness-materialization-compatibility-policy
kind: story
stage: review
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

## Implementation notes

- Files changed: `crates/core/src/compatibility.rs`, `crates/core/src/lib.rs`.
- Tests added: required/optional capability loss, target identity collision,
  supported capability, and dependency fixture coverage.
- Discrepancies from design: unknown component kinds use the same typed
  capability-unknown evidence path as missing registry entries, preserving a
  single fail-closed policy without inventing equivalences.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core compatibility::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.
