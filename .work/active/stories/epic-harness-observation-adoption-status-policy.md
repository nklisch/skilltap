---
id: epic-harness-observation-adoption-status-policy
kind: story
stage: implementing
tags: [cli,infra]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-normalization]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Manage Harness Policy

Implement strict skilltap harness policy load and deterministic list/enable/
disable operations. Missing policy remains explicit and read-only until enable;
enable creates only skilltap config with the named harness, disable edits only
policy, and unknown/duplicate/disabled selections fail safely without touching
native state.
