---
id: epic-safe-update-automation-resolution-contract
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-resolution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
