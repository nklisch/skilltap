---
id: gate-patterns-3-1-0
kind: story
stage: done
tags: [patterns]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: patterns
created: 2026-04-02
updated: 2026-07-15
---

# Patterns extracted for 3.1.0

## New patterns codified
- `drift-checked-managed-projection-plan` — plan skill trees and MCP configuration as fingerprinted writes, re-observe owned projections, and fail on drift.

## Inconsistencies flagged
- Shared managed-projection planning is duplicated across the file-managed adapter family.
- Codex performs equivalent drift checks inline rather than through the shared verification shape.

## Pattern files written
- `.agents/skills/patterns/drift-checked-managed-projection-plan.md`
- `.agents/skills/patterns/SKILL.md` (updated index)
- `.agents/rules/patterns.md` (regenerated hook-loaded digest)
- `.claude/skills/patterns` resolves through the canonical `.agents/skills` compatibility link.
