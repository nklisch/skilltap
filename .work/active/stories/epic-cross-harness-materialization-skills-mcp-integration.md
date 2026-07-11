---
id: epic-cross-harness-materialization-skills-mcp-integration
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-skills-mcp
depends_on: [epic-cross-harness-materialization-skills-mcp-mcp]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Integrate Skill and MCP Projection Plans

Compose target-bound skill and MCP projection plans from compatibility and
materialization results without publication or state mutation.

Acceptance criteria:

- Excluded components never reappear in projection output.
- Projection ordering and target identity are deterministic.
- Mapping failures stop before any publication boundary.

## Implementation notes

- Files changed: `crates/core/src/materialization.rs`.
- Tests added: composed skill/MCP projection plan with included-component
  filtering and deterministic ordering.
- Discrepancies from design: integration forwards MCP mapping through the core
  trait and leaves absolute roots/publication to the downstream publish
  feature, preserving the no-write boundary.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core materialization::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
