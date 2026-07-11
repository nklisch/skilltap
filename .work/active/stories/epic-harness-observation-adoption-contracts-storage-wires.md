---
id: epic-harness-observation-adoption-contracts-storage-wires
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-graph, epic-harness-observation-adoption-contracts-managed-ownership]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Reset Inventory and State Wires to Exact Keys

Migrate inventory/state maps, resource state, schema errors, managed ownership,
strict goldens, repositories, and storage integration to nested scope-bearing
keys. Keep independent schema constants at 1 for unreleased clean-break v3;
reject old ResourceId-only shapes with no migration or compatibility path.
