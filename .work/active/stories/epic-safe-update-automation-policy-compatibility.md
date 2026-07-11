---
id: epic-safe-update-automation-policy-compatibility
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-policy
depends_on: [epic-safe-update-automation-policy-contract]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Re-evaluate Compatibility for Updates

Compare target-bound compatibility analyses before and after a resolved update
and produce a typed change summary for safe-update classification.

Acceptance criteria:

- New required components block automatic application.
- New optional partial consequences require acknowledgment.
- Identical analyses produce no compatibility change.
