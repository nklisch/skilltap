---
id: epic-safe-update-automation-policy-contract
kind: story
stage: implementing
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
