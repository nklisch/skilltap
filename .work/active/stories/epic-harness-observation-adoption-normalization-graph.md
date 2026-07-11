---
id: epic-harness-observation-adoption-normalization-graph
kind: story
stage: review
tags: [infra]
parent: epic-harness-observation-adoption-normalization
depends_on: [epic-harness-observation-adoption-codex, epic-harness-observation-adoption-claude]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Compose Normalized Observation Graphs

Compose successful Codex/Claude observations into deterministic typed graphs
preserving exact harness, scope, resource, layer, source, and native identity.
Retain healthy siblings when other harness/scope observations fail and keep the
snapshot ephemeral and read-only.

## Implementation

- Added `normalize_observations` composition over the domain's deterministic
  `ObservationBatch`/`ObservedEnvironment` contracts, preserving every target
  and outcome without writes or payload reinterpretation.
- Added a deterministic ephemeral normalization smoke test; domain graph tests
  continue to cover duplicate/missing/unexpected sibling boundaries.

## Verification

- Harness Clippy and the normalization smoke test plus the locked workspace
  suite pass.
