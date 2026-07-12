---
id: epic-real-harness-recovery-state-diagnostics-update-eligibility
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Count only actionable available updates

## Scope

Give update decisions one authoritative actionable-summary predicate and use
it for status counts. Keep blocked and unresolved diagnostics visible while
preventing local instructions and non-resolvable local skills from appearing
as available updates.

## Acceptance

- Unresolved, blocked, disabled, and unchanged candidates contribute zero to
  `available_updates` while retaining their status and reason.
- Resolved changed safe and decision-required candidates are counted.
- Local instructions and local-path skills do not create phantom updates.
- Target-specific revision disagreement yields exact entries and identical
  plain/JSON summary counts.
