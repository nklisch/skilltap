---
id: epic-real-harness-recovery-state-diagnostics-target-evidence
kind: story
stage: review
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

## Implementation notes

- Execution capability: strongest available; this was an atomic, machine-state
  schema migration spanning strict storage, lifecycle journaling, publication,
  update recording, target removal, and status projection.
- Review weight: standard, inherited from the autopilot caller.
- Files changed: the state schema and strict fixture in
  `crates/core/src/storage/`, core publication/update/reconciliation call sites,
  CLI lifecycle/instruction/status/journal call sites, and their isolated core
  and compiled-binary tests.
- Tests added: dual native/managed target bindings with distinct lifecycle
  evidence; exact target projection; unknown, duplicate, mismatched-key, old
  schema, ownership, role, and artifact rejection; selected-target update
  journal clearing with byte-identical sibling preservation; compiled targeted
  update/removal and managed-project lifecycle regressions.
- Discrepancies from design: the strict wire represents bindings as
  `{ target, binding }` entries so deserialization can independently reject
  duplicate target keys and key/binding mismatches. During integration, the
  already-landed managed Codex project and typed diagnostics changes also
  required fixing the managed materialization fidelity contract, successful
  no-op result classification, and absent project-scope removal behavior; root
  explicitly assigned those shared code/test hunks to this atomic commit.
- Adjacent issues parked: none.
