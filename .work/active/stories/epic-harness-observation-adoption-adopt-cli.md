---
id: epic-harness-observation-adoption-adopt-cli
kind: story
stage: implementing
tags: [cli]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-persistence]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Adoption CLI

Route `adopt` through exact scope/target selection and the locked application
service. Render typed adopted/coalesced/already-managed/conflict/unadoptable
decisions in stable plain/JSON output; partial/conflict results require the
documented acknowledgment and no generic bypass is introduced.
