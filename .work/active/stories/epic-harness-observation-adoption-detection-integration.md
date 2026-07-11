---
id: epic-harness-observation-adoption-detection-integration
kind: story
stage: implementing
tags: [testing,infra]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-detection-fixtures, epic-harness-observation-adoption-detection-registry, epic-harness-observation-adoption-detection-profiles, epic-harness-observation-adoption-detection-probes]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Harness Detection End to End

Exercise Codex and Claude detection against isolated scripted natives and real
runtime boundaries. Cover sibling failure isolation, known/unknown versions,
compiled profile authority, probe narrowing and drift, malformed/duplicate
JSON, timeout/output limits, replacement races, secret-safe diagnostics,
repeat determinism, and no native/state mutation. Run the locked workspace
ladder and native Linux/macOS behavior suites; make only final export or
composition corrections here.
