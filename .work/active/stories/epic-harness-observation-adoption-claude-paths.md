---
id: epic-harness-observation-adoption-claude-paths
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-claude
depends_on: [epic-harness-observation-adoption-detection, epic-harness-observation-adoption-runtime]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/claude.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Derive Claude Observation Paths

Define bounded Claude user/global and one personal project roots for settings,
plugins, caches, and standalone skills. Resolve only documented paths, reject
unsafe roots, preserve project/shared distinctions, and never create or scan
unconfigured directories.

## Implementation

- Added `ClaudeObservationPaths` and `claude_observation_paths` for bounded
  user/global settings, plugin, skills, and personal project inputs.
- Added path-policy coverage proving global and project roots remain separate
  and no directories are created during derivation.

## Verification

- Harness Clippy and all ten detection/Codex/Claude path tests pass in the
  locked offline workspace.

## Review

- Fast-lane review approved the bounded read-only Claude path derivation.
