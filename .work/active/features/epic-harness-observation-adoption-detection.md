---
id: epic-harness-observation-adoption-detection
kind: feature
stage: drafting
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-contracts, epic-harness-observation-adoption-runtime]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Harness Detection and Capability Profiles

Build the concrete adapter registry, Codex/Claude installation and opaque
version detection, compiled verified scope-aware profile selection, and
read-only probe narrowing. Profiles are the mutation allowlist; help text is
negative evidence only, JSON success must pass its parser contract, and unknown
versions never gain mutation authority. Provide reusable scripted fake-native
fixtures for missing/unexecutable binaries, output/timeout failures, known and
unknown versions, probe drift, and executable replacement. Do not yet observe
resources or expose CLI commands.
