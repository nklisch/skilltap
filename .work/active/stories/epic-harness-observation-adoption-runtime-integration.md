---
id: epic-harness-observation-adoption-runtime-integration
kind: story
stage: implementing
tags: [testing,infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-executable-resolution, epic-harness-observation-adoption-runtime-bounded-process, epic-harness-observation-adoption-runtime-strict-json, epic-harness-observation-adoption-runtime-codex-home, epic-harness-observation-adoption-runtime-external-tree]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify the Complete Native Observation Runtime

Exercise resolve identity -> bounded direct run -> strict typed JSON as one
pipeline, alongside isolated `CODEX_HOME` resolution and bounded external-tree
snapshots. Prove repeat determinism, zero mutation, safe failure rendering,
boundary limits, executable/tree replacement handling, process-group escape
with retained pipes, descendant termination,
and output secret canaries with adversarial fixtures. Run the full locked Rust
ladder, optimized compiled-binary verification, and native Linux/macOS behavior
jobs; make only final export/composition corrections here.
