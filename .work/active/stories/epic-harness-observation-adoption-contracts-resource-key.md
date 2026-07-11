---
id: epic-harness-observation-adoption-contracts-resource-key
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Add Scope-Bearing Resource Keys

Add validated `ResourceKey { id, scope }`, canonical encoding, serde/order/hash
contracts, and a ResourceId-specific alphabet supporting documented qualified
plugin spelling without relaxing other identifiers. Prove equal IDs across
global and multiple projects remain distinct and malformed qualified IDs fail.
