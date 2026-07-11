---
id: epic-harness-observation-adoption-contracts-findings
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-graph]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Make Observation Findings Safe by Construction

Replace fixed coarse finding kinds plus arbitrary messages/JSON metadata with
validated open codes, authored static summaries, severity, typed subjects, and
a bounded scalar field vocabulary. Add secret canaries proving raw argv,
stdout/stderr, settings, unknown JSON, and dynamic messages cannot enter domain
findings or their Debug/Display/serde forms.
