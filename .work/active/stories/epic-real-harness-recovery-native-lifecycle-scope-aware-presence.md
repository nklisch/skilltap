---
id: epic-real-harness-recovery-native-lifecycle-scope-aware-presence
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Match native resource presence by concrete scope

## Finding

Claude list commands omit `--scope` and return entries carrying their own
scope. The current parser compares only identity, so an equal global resource
can be mistaken for the requested project/local resource during journal
re-observation.

## Required fix

- Project list observations by both requested identity and the exact native
  scope represented by the request; global maps to `user` and personal project
  scope maps to the attested `local` value.
- Treat missing, malformed, duplicate, or contradictory scope evidence as
  unknown rather than borrowing evidence from another scope.
- Preserve Codex behavior only where its attested list shape is genuinely
  scope-unambiguous.
- Add adapter and compiled lifecycle regressions where a same-name global
  resource coexists with a missing/drifted project resource and cannot satisfy
  the project re-observation.

## Acceptance

- Native presence evidence never crosses a concrete global/project boundary.
- A removed project resource is reapplied even when its global sibling exists.
- Malformed scope evidence fails closed without exposing raw payloads.
