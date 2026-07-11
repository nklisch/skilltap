---
id: epic-harness-observation-adoption-detection-profiles
kind: story
stage: done
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

## Implementation

- Added immutable Codex and Claude v3 compiled profiles with scope-aware
  global/project capabilities. Known `3.0.0` versions receive verified
  profile ids; every other version receives an observe-only profile with
  unverified observation capabilities and no mutation authority.
- Added profile-selection tests proving mutation authority is present only for
  known compiled versions and absent for unknown versions.

## Verification

- Harness detection Clippy and all four detection tests pass in the locked
  offline workspace.

## Review

- Fast-lane review approved the deterministic profile selection and green
  warnings-denied tests. Unknown versions cannot acquire mutation authority.
