---
id: epic-safe-update-automation-policy-status
kind: story
stage: done
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

## Implementation Notes

- Status now uses `classify_update_with_mode` and renders bounded decision
  reasons for disabled, policy-off, check-only, pinned, blocked, and safe
  candidates.
- Global `off` and disabled resources skip revision resolution entirely;
  `check` resolves and reports but never appears safe for mutation.
- Resolution failures remain visible as blocked update entries alongside their
  existing safe warning context.
- Verification: CLI tests and clippy passed.

## Review Record

- Inline review: **pass**. Plain and JSON output share the same typed status
  projection, and status remains read-only.
