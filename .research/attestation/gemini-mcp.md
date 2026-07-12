---
source_handle: gemini-mcp
fetched: 2026-07-12
source_url: https://geminicli.com/docs/tools/mcp-server/
provenance: source-direct
---

# Gemini CLI MCP

Gemini CLI reads named MCP servers from the `mcpServers` object in user and
project `settings.json` files. Its shell commands add, remove, list, enable, and
disable servers with user/project scope. It supports stdio, HTTP, and SSE,
reports connection failures through `gemini mcp list`, and provides `/mcp
reload` for capability refresh.

## Key passages

- The setup section defines the `mcpServers` schema and supported transports.
- `gemini mcp remove` documents the `user` or `project` scope selector.
- Troubleshooting names `gemini mcp list`; the tutorial names `/mcp reload`.
- The settings documentation identifies `~/.gemini/settings.json` and project
  `.gemini/settings.json` as merged sources.
