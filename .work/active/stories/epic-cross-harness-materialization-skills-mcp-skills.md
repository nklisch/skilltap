---
id: epic-cross-harness-materialization-skills-mcp-skills
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-skills-mcp
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Plan Complete Portable Skill Projections

Implement complete-tree skill projection planning in
`crates/core/src/materialization.rs` using source provenance and documented
Codex/Claude roots.

Acceptance criteria:

- Every included skill has a top-level `SKILL.md` and a deterministic complete
  destination plan.
- Excluded, malformed, or provenance-less skills fail closed.
- Planning performs no filesystem writes.
