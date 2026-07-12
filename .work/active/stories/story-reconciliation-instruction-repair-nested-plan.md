---
id: story-reconciliation-instruction-repair-nested-plan
kind: story
stage: implementing
tags: [correctness, testing]
parent: null
depends_on: [story-reconciliation-instruction-repair]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile nested-only project instruction bridges accurately

## Finding

`instructions setup --project` intentionally preserves a supported nested
Claude bridge at `<project>/.claude/CLAUDE.md` when the project-root
`CLAUDE.md` is absent. Reconciliation preview currently always inspects the
root path returned by `instruction_locations`, so `plan --project --target
claude` reports `repair` for a healthy nested-only bridge. `sync` then performs
no change, making plan output inaccurate and violating the instruction bridge
contract.

## Required behavior

- Resolve a nested-only project Claude bridge using the same location policy as
  setup/repair before classifying plan state.
- Report `no_change` for a managed nested bridge and `repair`/`blocked` only
  for its actual state.
- Add isolated compiled coverage for nested-only project plan and sync
  idempotence, including both symlink and import modes if practical.

## Review evidence

The existing setup regression `instruction_setup_preserves_existing_nested_project_claude_bridge`
demonstrates the supported state. `execute_instruction_reconciliation_preview`
currently selects only the root project bridge and has no nested fallback.
