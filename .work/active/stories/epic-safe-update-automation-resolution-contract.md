---
id: epic-safe-update-automation-resolution-contract
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-resolution
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Define Typed Update Resolution

Implement the core contracts in `crates/core/src/updates.rs`: typed revisions,
resolver ports, bounded errors, target-aware candidate construction, and safety
classification.

Acceptance criteria:

- No stringly typed revision comparison remains in update policy.
- Pinned, drifted, incompatible, and partial candidates cannot be classified
  as safe by accident.
- Resolver errors are deterministic, bounded, and covered by unit tests.

## Implementation notes

- Files changed: `crates/core/src/updates.rs`.
- Tests added: typed revision safety, source resolution, and native target
  disagreement tests in `updates.rs`.
- Discrepancies from design: `ResolvedUpdate` retains the three planned fields
  directly and `candidate_for` derives the policy candidate; unresolved results
  remain explicit through `error` rather than being classified as no-update.
- Adjacent issues parked: none.

## Verification

- `cargo test --workspace --all-targets --offline` — passed (all workspace
  tests).
- `cargo clippy --workspace --all-targets --offline -- -D warnings` — passed
  for the touched core crate before the workspace verification pass.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
