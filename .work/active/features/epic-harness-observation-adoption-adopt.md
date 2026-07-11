---
id: epic-harness-observation-adoption-adopt
kind: feature
stage: drafting
tags: []
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-status]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Conflict-Aware Adoption

Implement pure adoption candidate, coalescing, equivalence, conflict, and
idempotence contracts over the shared fresh snapshot. Under the configuration
lock, reload inventory, revalidate selected identities/fingerprints, preserve
manual/unrelated entries, add every non-conflicting candidate with adopted
source provenance, and publish one atomic inventory replacement. Single-source
resources target their source harness; already equivalent multi-harness
resources may coalesce. Shared Claude project declarations remain unadoptable.
Adoption never calls a native mutation, writes observation state, transfers a
resource, or discards healthy siblings because another conflicts.
