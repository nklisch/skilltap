---
source_handle: cursor-mcp
fetched: 2026-07-12
source_url: https://docs.cursor.com/context/model-context-protocol
provenance: source-direct
---

# Cursor MCP

Cursor documents project MCP configuration at `.cursor/mcp.json` and global
configuration at `~/.cursor/mcp.json`. The same JSON supports stdio, SSE, and
streamable HTTP, and Cursor's CLI exposes list and per-server tool inspection.
OAuth and an extension registration API are also supported.

## Key passages

- “Configuration locations” names project and global files.
- “Using `mcp.json`” defines the JSON form.
- “Protocol support” and the transport table describe supported MCP behavior.
- The Cursor CLI reference documents `cursor-agent mcp list` and `list-tools`.
