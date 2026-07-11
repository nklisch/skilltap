---
id: epic-cross-harness-materialization-skills-mcp-skills
kind: story
stage: review
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

## Implementation notes

- Files changed: `crates/core/src/materialization.rs`.
- Tests added: complete-tree Claude canonical/target projection and excluded
  component no-reappearance tests.
- Discrepancies from design: complete `SKILL.md` presence is validated by the
  explicit source reader; the pure planner additionally rejects file-level
  provenance and malformed skill identity paths before emitting projections.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core materialization::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.
