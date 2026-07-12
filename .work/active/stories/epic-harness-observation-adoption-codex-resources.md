---
id: epic-harness-observation-adoption-codex-resources
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-codex
depends_on: [epic-harness-observation-adoption-codex-config]
release_binding: 3.0.0
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/codex.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Observe Codex Resources and Instructions

Build layered Codex observations for marketplace/plugins, complete skill
directories containing top-level `SKILL.md`, global/project instructions, and
effective cache/manifests. Track conformance/loadability, declared versus
effective plugin state, source/provenance, and `AGENTS.override.md` precedence
without reading outside bounded native trees or emitting raw bytes.

## Implementation

- Added bounded `observe_codex_resources` composition over the descriptor-
  relative external-tree observer. Complete skill directories and their
  top-level `SKILL.md` remain native tree evidence; the adapter performs no
  writes or cache materialization.
- Added a no-mutation integration test for a complete Codex skill directory.

## Verification

- Harness Clippy and all nine detection/Codex path/config/resource tests pass
  in the locked offline workspace.

## Review

- Fast-lane review approved the no-mutation complete-tree observation and green
  warnings-denied test record.
