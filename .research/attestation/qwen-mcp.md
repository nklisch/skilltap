---
source_handle: qwen-mcp
fetched: 2026-07-12
source_url: https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/
provenance: source-direct
---

# Qwen Code MCP

Qwen loads MCP servers from `mcpServers` in `settings.json`. The default user
scope is `~/.qwen/settings.json`; project scope is `.qwen/settings.json` at the
project root. `qwen mcp add` accepts an explicit user or project scope and
supports stdio, HTTP, and SSE. A running session may need to be restarted in
the project after configuration changes.

## Key passages

- “Where configuration is stored (scopes)” names user and project files.
- “Quick start” documents `qwen mcp add` and the `/mcp` management surface.
- “Choose a transport” lists HTTP, SSE, and stdio configuration forms.
- The reload note says to restart Qwen in the same project after adding a server.
