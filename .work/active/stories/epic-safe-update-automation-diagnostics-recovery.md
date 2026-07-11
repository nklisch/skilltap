---
id: epic-safe-update-automation-diagnostics-recovery
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-diagnostics
depends_on: [epic-safe-update-automation-diagnostics-status]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Offer Deterministic Daemon Recovery Actions

Map typed daemon failures and pending results to bounded advisory next actions
without retrying or acknowledging anything automatically.

Acceptance criteria:

- Every non-completed result includes a safe next command.
- Recovery suggestions never mutate definitions/resources.
- Repeated diagnostics is idempotent.

## Implementation Notes

- Added deterministic recovery next actions for pending, contended, failed,
  disabled, and never-run daemon states. Suggestions are advisory commands
  only; diagnostics performs no retry or acknowledgment.
- Recovery actions are rendered through the same plain/JSON outcome model.
- Verification: CLI tests and clippy passed.

## Review Record

- Inline review: **pass**. Every non-completed state has a bounded next action
  and no recovery path mutates resources.
