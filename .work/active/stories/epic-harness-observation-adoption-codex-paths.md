---
id: epic-harness-observation-adoption-codex-paths
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-codex
depends_on: [epic-harness-observation-adoption-detection, epic-harness-observation-adoption-runtime]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/codex.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Derive Codex Observation Paths

Define the bounded Codex global and one canonical project input roots from
`CODEX_HOME`, scope policy, and documented native contracts. Resolve only
configured files/directories, reject unsafe roots, preserve `~/AGENTS.md` and
project override precedence, and never create or scan paths.
