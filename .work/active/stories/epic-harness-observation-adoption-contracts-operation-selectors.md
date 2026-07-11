---
id: epic-harness-observation-adoption-contracts-operation-selectors
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-graph]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Migrate Exact Operation Selectors

Move resource/component operation selectors, acknowledgment sets, consequence
coverage, and errors/wires to exact `ResourceKey`. Enforce selector scope equals
operation semantic scope and preserve all partial/blocked dependency behavior.
