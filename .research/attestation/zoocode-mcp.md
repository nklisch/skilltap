---
source_handle: zoocode-mcp
fetched: 2026-07-12
source_url: https://docs.zoocode.dev/features/mcp/using-mcp-in-roo
provenance: source-direct
---

# Zoo Code MCP

Zoo Code supports a global `mcp_settings.json` opened through its settings UI
and project `.roo/mcp.json`, with project definitions taking precedence. Both
files use an inspectable `mcpServers` JSON structure. Zoo supports stdio,
streamable HTTP, SSE, enablement, and per-tool policy.

## Key passages

- “Configuring MCP Servers” defines both scopes and precedence.
- “Editing MCP Settings Files” opens both direct file surfaces.
- Transport sections define stdio, streamable HTTP, and SSE.
- Enablement changes are represented in the same configuration model.
