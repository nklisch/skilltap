---
id: epic-harness-observation-adoption-detection-profiles
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-detection-registry]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Select Compiled Capability Profiles

Add immutable Codex/Claude version profile catalogues and deterministic
scope-aware selection. Known versions receive exactly one verified compiled
profile and mutation authority; unknown versions remain observation-valid but
observe-only with no profile id or mutation capabilities. Enforce capability
set boundaries, profile/version mismatch rejection, and safe serialization and
Debug output at every constructor and deserialization boundary.
