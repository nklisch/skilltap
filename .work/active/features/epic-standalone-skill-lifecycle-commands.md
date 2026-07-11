---
id: epic-standalone-skill-lifecycle-commands
kind: feature
stage: drafting
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-storage, epic-standalone-skill-lifecycle-compatibility]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Skill Lifecycle Commands

Compose explicit skill install/list/remove/update commands with exact scopes,
target projections, compatibility gates, and Git SHA update tracking.

## Design

- Install and update resolve a fresh tree, compare resolved SHA and whole-tree
  fingerprint, then plan before mutation.
- Pins suppress automatic update but never hide drift; foreground operations
  can override only with an explicit operation-scoped acknowledgment.
- Remove requires skilltap ownership and leaves unmanaged or drifted content
  untouched unless the plan is explicitly accepted.
- List reports desired/installed state only; it never searches sources or
  marketplace contents.

## Acceptance

All lifecycle commands are non-interactive, deterministic in plain/JSON mode,
and an immediate repeat is a no-op.
