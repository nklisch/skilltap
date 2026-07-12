---
source_handle: kiro-mcp
fetched: 2026-07-12
source_url: https://kiro.dev/docs/cli/mcp/configuration/
provenance: source-direct
---

# Kiro CLI MCP

Kiro CLI loads MCP JSON from global `~/.kiro/settings/mcp.json` and workspace
`.kiro/settings/mcp.json`, with workspace definitions above global definitions.
The CLI supports local and remote servers, disablement and tool filtering. MCP
files hot-reload: only changed servers restart, and `/mcp` reports connection
state and tools.

## Key passages

- “MCP server loading priority” names both supported files.
- “Hot-reload” documents file watching and idempotent order-independent diffs.
- “Viewing loaded servers” documents the `/mcp` status surface.
- The schema separates command-based and URL-based servers.
