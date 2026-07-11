---
id: epic-cross-harness-materialization-skills-mcp-mcp
kind: story
stage: review
tags: []
parent: epic-cross-harness-materialization-skills-mcp
depends_on: [epic-cross-harness-materialization-skills-mcp-skills]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Map Conditional MCP Projections

Implement strict MCP projection mapping in core/harness adapters for documented
transport, auth, variable, and load-path semantics without copying secrets.

Acceptance criteria:

- Supported stdio/HTTP fixture declarations preserve semantic references.
- Unsupported transports or auth return typed consequences.
- Cache paths and credential values never become projection or state data.

## Implementation notes

- Files changed: `crates/core/src/materialization.rs`,
  `crates/harnesses/src/materialization.rs`, and
  `crates/harnesses/src/lib.rs`.
- Tests added: named HTTP MCP reference mapping and literal-credential /
  ambiguous-transport blocking fixtures.
- Discrepancies from design: the mapper emits only relative MCP destination
  metadata and validates command/URL semantics; absolute target roots and
  publication remain with the downstream publish feature.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-harnesses materialization::tests --offline` — passed.
- `cargo clippy -p skilltap-harnesses --all-targets --offline -- -D warnings`
  — passed.
