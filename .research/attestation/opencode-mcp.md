---
source_handle: opencode-mcp
fetched: 2026-07-12
source_url: https://dev.opencode.ai/docs/mcp-servers/
provenance: source-direct
---

# OpenCode MCP

OpenCode configures local and remote MCP servers under the `mcp` key in its
regular configuration. The configuration contract layers global
`~/.config/opencode/opencode.json` and a project-root `opencode.json`, with
project values overriding global ones. MCP list/auth/debug commands expose
configured and runtime state, while OAuth tokens are stored separately.

## Key passages

- The local and remote sections define supported MCP schema.
- “Manage” documents enabled/disabled tool behavior.
- The configuration guide names global and per-project files and precedence.
- The authentication section documents `opencode mcp list` and `mcp debug`.
