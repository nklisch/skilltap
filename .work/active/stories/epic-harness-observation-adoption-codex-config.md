---
id: epic-harness-observation-adoption-codex-config
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-codex
depends_on: [epic-harness-observation-adoption-codex-paths]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/codex.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Parse Codex Configuration Evidence

Observe documented Codex config, marketplace declarations, trust/project
settings, and malformed siblings with strict bounded decoding and safe finding
categories. Preserve unknown native fields, distinguish declared from effective
state, and retain successful siblings when one document is missing, malformed,
or replaced. No writes, cache browsing, or guessed install behavior.
