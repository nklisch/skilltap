---
id: epic-cross-harness-materialization-skills-mcp-integration
kind: story
stage: implementing
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
