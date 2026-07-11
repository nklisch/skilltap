---
id: epic-safe-update-automation-policy-contract
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-policy
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Add Typed Safe-Update Decisions

Represent global update modes and per-resource update intent in a pure typed
decision. Keep disabled, pinned, drifted, compatibility, acknowledgment, and
resolution failures distinct.

Acceptance criteria:

- Only a clean tracked candidate in `apply-safe` is automatically safe.
- `off` and `check` never authorize automatic mutation.
- Resolver errors remain blocked and carry a deterministic reason.

## Implementation Notes

- Added `UpdateDecision`, `UpdateDecisionReason`, and
  `classify_update_with_mode` to the core update boundary.
- Candidates now retain `UpdateIntent`; disabled resources are no-op decisions,
  pins require a foreground decision, `off`/`check` never authorize safe
  application, and resolution failures remain blocked.
- Verification: targeted update tests and core clippy passed.

## Review Record

- Inline review: **pass**. Policy classification is deterministic and does not
  infer safety from revision distance.
