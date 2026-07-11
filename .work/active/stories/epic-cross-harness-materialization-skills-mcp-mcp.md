---
id: epic-cross-harness-materialization-skills-mcp-mcp
kind: story
stage: implementing
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
