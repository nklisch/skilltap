---
id: epic-safe-update-automation-resolution-orchestration
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-resolution
depends_on: [epic-safe-update-automation-resolution-adapters]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Check and Cache Available Revisions

Wire the pure resolution contracts into the application status/check path and
add atomic available-revision caching in `state.json` without mutating desired
inventory, managed artifacts, or native harness files.

Acceptance criteria:

- Repeating an unchanged check is a no-op and reports no update.
- Changed Git SHAs and native revisions are visible before any update action.
- Failures leave state, inventory, and native configuration unchanged.
- Successful cache writes preserve existing operation journals and emit the
  documented human/JSON next actions.
