---
source_handle: kimi-mcp
fetched: 2026-07-12
source_url: https://www.kimi.com/code/docs/en/kimi-code-cli/customization/mcp.html
provenance: source-direct
---

# Kimi Code MCP

Kimi Code loads MCP JSON from user `$KIMI_CODE_HOME/mcp.json` (normally
`~/.kimi-code/mcp.json`) and project `.kimi-code/mcp.json`. Project entries
override same-named user entries. The schema supports stdio, HTTP, SSE,
enablement, timeouts, and tool filters. `/mcp` reports connection state; plugin
MCP changes require a new session.

## Key passages

- “Configuration” names both paths and precedence.
- The schema lists transport and enablement fields.
- `/mcp` is the documented status surface.
- The plugin note says plugin-provided MCP servers need a new session after
  enablement changes.
