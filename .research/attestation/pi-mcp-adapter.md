---
source_handle: pi-mcp-adapter
fetched: 2026-07-12
source_url: https://pi.dev/packages/pi-mcp-adapter
provenance: source-direct
---

# Pi MCP adapter package

The Pi package catalog hosts `pi-mcp-adapter`, an optional extension that adds
MCP support. It reads user-global `~/.config/mcp/mcp.json` and Pi override
`~/.pi/agent/mcp.json`, plus project `.mcp.json` and `.pi/mcp.json`, with a
documented precedence order. It exposes `/mcp` and an MCP tool for effective
state, and stores tool metadata in a cache that is observational rather than a
configuration write target.

## Key passages

- “Quick Start” and “Config” name global and project files.
- The precedence list defines resolution across all four files.
- `/mcp` and the `mcp` tool expose loaded state.
- The metadata cache is described separately from supported configuration files.
