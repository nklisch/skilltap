---
id: epic-real-harness-recovery-state-diagnostics-target-evidence
kind: story
stage: implementing
tags: [correctness, architecture, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Persist lifecycle evidence per target

## Scope

Replace ambiguous resource-wide lifecycle facts with one validated target
binding map while retaining only the logical resource key above the bindings.
Update storage, publication, foreground update recording, and strict wire
fixtures as one atomic contract change.

## Acceptance

- Codex and Claude bindings for one logical resource may carry distinct native
  IDs, sources, revisions, provenance, ownership, artifacts, and timestamps.
- Native and managed sibling representations validate without a resource-wide
  ownership or provenance claim.
- Each binding owns its exact apply journal; target projection and verified
  update recording preserve all unselected sibling evidence and remove stale
  selected-target journal evidence.
- Strict serde DTOs reject old-schema, unknown, duplicate, mismatched-key, and
  invalid provenance/ownership/artifact state.
- Storage, publication, update, and integration tests use only the new target
  accessors and the strict golden round trips.
