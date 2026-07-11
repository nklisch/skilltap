---
id: epic-harness-observation-adoption-adopt-merge
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-candidates]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Merge Adoption Decisions

Add conservative equivalence, cross-harness coalescing, conflict isolation,
stable adopted provenance, deterministic ordering, and inventory merge helpers.
Preserve manual/unrelated desired resources and make repeated merges a logical
no-op.
