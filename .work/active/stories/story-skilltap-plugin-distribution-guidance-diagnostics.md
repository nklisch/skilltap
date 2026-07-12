---
id: story-skilltap-plugin-distribution-guidance-diagnostics
kind: story
stage: implementing
tags: [content]
parent: epic-skilltap-plugin-distribution-guidance
depends_on: [story-skilltap-plugin-distribution-guidance-core]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Document diagnostic, update, and recovery decisions

Add `plugin/skills/skilltap/references/diagnostics.md` as a progressive
reference for agents explaining status/plan/sync output, attention and partial
results, next actions, binary bootstrap outcomes, Git-SHA update tracking, and
the optional daemon. The reference must tell the agent when to stop and ask the
user rather than inventing a bypass.

Acceptance criteria:

- Healthy, changes-needed, attention, partial, blocked, and unavailable states
  each map to a user-facing explanation and safe next action.
- Same-major binary updates, opt-out, explicit major acknowledgment, native
  plugin updates, and Git-SHA skill updates are kept distinct.
- Daemon behavior is described as safe-update automation that never acknowledges
  partial operations or overwrites drift.
- Plain and JSON output are presented as equivalent semantics, with executable
  help authoritative for exact fields and syntax.

## Implementation notes
- Execution capability: standard; prose follows stable output and update policy contracts.
- Review weight: standard (autopilot caller policy).
