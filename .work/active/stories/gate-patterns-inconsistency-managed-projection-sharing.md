---
id: gate-patterns-inconsistency-managed-projection-sharing
kind: story
stage: drafting
tags: [refactor]
parent: null
depends_on: []
release_binding: null
gate_origin: patterns
created: 2026-07-15
updated: 2026-08-15
---

# Converge the remaining managed adapters on shared projection planning

The `drift-checked-managed-projection-plan` pattern is centralized for AGY, Kimi, Kilo, Vibe, Amp, Junie, Copilot, and Factory, while Codex, OpenCode, Qwen, and Kiro retain private near-identical planning, observation, verification, fingerprint, and limit helpers.

Assess and migrate the remaining Codex and Qwen shapes to the narrowest shared abstraction that preserves their source-plugin inputs, diagnostics, codecs, manifests, fingerprint ordering, and drift behavior. This is behavior-preserving structural work.
