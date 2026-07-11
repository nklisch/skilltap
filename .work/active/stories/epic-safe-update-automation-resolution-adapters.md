---
id: epic-safe-update-automation-resolution-adapters
kind: story
stage: review
tags: []
parent: epic-safe-update-automation-resolution
depends_on: [epic-safe-update-automation-resolution-contract]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve Git and Native Revisions Without Mutation

Implement `crates/harnesses/src/update_resolution.rs` with bounded Git ref
resolution and native observation-backed revision resolution. Do not install,
update, checkout, or write caches during this story.

Acceptance criteria:

- Explicit Git refs resolve to validated commit SHAs in fixture repositories.
- Unreachable, malformed, ambiguous, local, and unsupported sources return
  typed errors.
- Native resolution uses fresh verified observation only and handles unknown
  harness versions conservatively.

## Implementation notes

- Files changed: `crates/harnesses/src/update_resolution.rs`,
  `crates/harnesses/src/lib.rs`.
- Tests added: strict single-SHA Git output parsing and malformed/ambiguous
  output rejection.
- Discrepancies from design: the Git resolver is generic over the existing
  `NativeProcessRunner` port and exposes a system constructor; native revision
  resolution reuses `ObservedEnvironment` and prefers effective observations.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-harnesses --offline` — passed.
- `cargo clippy -p skilltap-harnesses --all-targets --offline -- -D warnings`
  — passed.
