---
id: epic-safe-update-automation-foreground-plan
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-foreground
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Assemble Foreground Update Plans

Pair exact desired resources with resolved update candidates and emit a
deterministic plan of safe operations plus typed blocked/needs-decision
findings.

Acceptance criteria:

- Clean tracked candidates produce stable update operations.
- Blocked, pinned, drifted, and unresolved candidates produce no mutation.
- Resource scope and target identity remain exact in every finding.
