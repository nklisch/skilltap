---
id: epic-harness-observation-adoption-normalization-findings
kind: story
stage: implementing
tags: [correctness]
parent: epic-harness-observation-adoption-normalization
depends_on: [epic-harness-observation-adoption-normalization-graph]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Preserve Normalization Findings

Retain malformed siblings, unresolved dependencies, partial harness failures,
unsupported/ambiguous lineage, and health evidence as deterministic typed
findings attached to surviving observations. Never collapse a partial snapshot
into global failure or leak native payloads.
