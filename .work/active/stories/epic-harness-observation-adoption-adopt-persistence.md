---
id: epic-harness-observation-adoption-adopt-persistence
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-merge]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Adoption Atomically

Acquire the configuration lock fail-fast, reload inventory, revalidate
selected observation identity/fingerprint evidence, rerun the pure plan, and
publish one atomic inventory replacement. Preserve unrelated entries and leave
native configuration, state.json, and managed artifacts untouched.
