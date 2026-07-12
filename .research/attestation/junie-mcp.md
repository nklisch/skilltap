---
source_handle: junie-mcp
fetched: 2026-07-12
source_url: https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html
provenance: source-direct
---

# Junie CLI MCP

Junie CLI loads MCP JSON from project `.junie/mcp/mcp.json` and user
`~/.junie/mcp/mcp.json`. Both are documented manual configuration surfaces.
The `/mcp` view reports each server's scope, starting/active/inactive/disabled/
failed/auth-required state, and permits enablement changes. Additional search
locations can be supplied through CLI options.

## Key passages

- “Add an MCP server” offers project and user installation scopes.
- “Add an MCP server from JSON configuration” permits direct editing.
- “List configured MCP servers” names both paths and observable states.
- `--mcp-location` and `--mcp-default-locations` control discovery.
