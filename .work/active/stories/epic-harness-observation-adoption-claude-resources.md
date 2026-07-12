---
id: epic-harness-observation-adoption-claude-resources
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-claude
depends_on: [epic-harness-observation-adoption-claude-settings]
release_binding: 3.0.0
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/claude.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Observe Claude Plugins, Skills, and Effective Cache

Build layered Claude observations for marketplace-installed plugins,
skills-directory plugins, standalone complete skill folders, enabled
declarations, effective cache installs, and version basis. Keep cache
loadability separate from declaration/consent authority and never emit raw
native bytes or mutate settings/cache.

## Implementation

- Added bounded `observe_claude_resources` composition over the external-tree
  observer for complete skills and cache/plugin evidence without writes.
- Added no-mutation integration coverage for a complete Claude skill folder.

## Verification

- Harness Clippy and all twelve detection/Codex/Claude path/settings/resource
  tests pass in the locked offline workspace.

## Review

- Fast-lane review approved the bounded complete-tree observation and green
  no-mutation test record.
