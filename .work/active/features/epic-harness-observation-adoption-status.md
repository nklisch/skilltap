---
id: epic-harness-observation-adoption-status
kind: feature
stage: drafting
tags: [cli]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-normalization]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Harness Management and First-Use Status

Replace CLI placeholders with `harness list`, `harness enable`, `harness
disable`, and observation-backed `status`. Missing config remains explicit:
status detects both known harnesses but reports neither user-enabled, while
enable creates config with only the named harness enabled and never touches
native state. Expand only requested global/project/inventory-recorded scopes,
report reachability/version/profile/capabilities/resources/findings and partial
sibling success, preserve JSON/plain/exit contracts, and prove every list/status
path creates or writes nothing.
