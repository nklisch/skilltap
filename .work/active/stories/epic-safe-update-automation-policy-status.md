---
id: epic-safe-update-automation-policy-status
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-policy
depends_on: [epic-safe-update-automation-policy-compatibility]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Project Safe-Update Decisions in Status

Use the core policy classifier in status output, distinguish disabled/pinned/
blocked/check-only/safe candidates, and preserve read-only behavior in plain
and JSON modes.

Acceptance criteria:

- Status renders a stable typed decision reason.
- `off` avoids source resolution and mutation; `check` reports without safe
  application.
- Plain and JSON projections remain derived from one outcome.
