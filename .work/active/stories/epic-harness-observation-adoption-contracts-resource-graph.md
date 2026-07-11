---
id: epic-harness-observation-adoption-contracts-resource-graph
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Migrate Resource Graph Identity

Make desired and observed resources, observation keys, graph maps/errors, and
dependencies use exact `ResourceKey`. Remove redundant observed scope, add
typed optional source, and retain resolved/unresolved observed dependency
evidence without aborting healthy siblings. Preserve deterministic wires and
validate exact self/dangling/cycle contexts.
