---
id: epic-harness-observation-adoption-runtime-codex-home
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve Codex Home Without Moving Global Instructions

Extend runtime path resolution so a non-empty normalized absolute `CODEX_HOME`
wins and absent/empty falls back to `$HOME/.codex`. Reject relative,
non-normalized, inaccessible, and non-UTF-8 values without rendering bytes.
Resolution creates nothing. XDG continues to relocate only skilltap state and
Codex-native paths never relocate canonical global `~/AGENTS.md`.
