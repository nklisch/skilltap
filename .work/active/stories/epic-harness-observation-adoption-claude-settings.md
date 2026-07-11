---
id: epic-harness-observation-adoption-claude-settings
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-claude
depends_on: [epic-harness-observation-adoption-claude-paths]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/claude.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Parse Claude Settings and Declarations

Parse bounded Claude settings and list operations, preserving unknown native
fields and qualified `name@marketplace` identities. Distinguish user/local/
project/managed declarations, shared project non-adoptability, malformed
siblings, and trust/consent/policy/bridge evidence with safe typed findings.

## Implementation

- Added bounded `observe_claude_settings` JSON parsing with qualified
  `name@marketplace` counting, enabled declaration counts, trust presence, and
  shared-project state. Unknown fields are tolerated and native values never
  enter returned Debug or findings.
- Added settings identity/shared-project and malformed/strict JSON coverage.

## Verification

- Harness Clippy and all eleven detection/Codex/Claude path/settings tests pass
  in the locked offline workspace.

## Review

- Fast-lane review approved the qualified identity/shared-project parsing and
  redacted strict JSON behavior.
