---
id: gate-patterns-3.0.2
kind: story
stage: done
tags: [patterns]
parent: null
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: patterns
created: 2026-07-12
updated: 2026-07-12
---

# Patterns extracted for 3.0.2

## New patterns codified

- `revalidated-execution-port` — bind and revalidate adapter requests under the shared lock.
- `root-confined-filesystem-port` — use canonical roots and no-follow relative operations.
- `target-local-resource-state` — preserve sibling harness bindings during target-local mutation.
- `identity-aware-rollback` — restore proven identities and report residual uncertainty.

## Inconsistencies flagged

No inconsistencies with the existing four-pattern catalog. The scanner found
one adoption defect in managed skill rollback; it is tracked separately as a
behavioral bug rather than mislabeled as a refactor.

## Pattern files written

- `.agents/skills/patterns/revalidated-execution-port.md`
- `.agents/skills/patterns/root-confined-filesystem-port.md`
- `.agents/skills/patterns/target-local-resource-state.md`
- `.agents/skills/patterns/identity-aware-rollback.md`
- `.agents/skills/patterns/SKILL.md` (updated index)
- `.agents/rules/patterns.md` (generated hook-loaded digest)
