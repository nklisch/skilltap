---
source_handle: mistral-mcp
fetched: 2026-07-12
source_url: https://docs.mistral.ai/vibe/code/cli/mcp-servers
provenance: source-direct
---

# Mistral Vibe MCP

Vibe defines MCP servers as `[[mcp_servers]]` tables in `config.toml` and
supports stdio, HTTP, and streamable HTTP. The companion configuration contract
loads project `./.vibe/config.toml` ahead of user `~/.vibe/config.toml`; project
configuration is honored only for trusted directories. `/mcp` lists a server's
tools. OAuth-backed MCP servers are a documented limitation.

## Key passages

- “Add an MCP server” defines the table and transports.
- “Known limitation” excludes MCP servers requiring OAuth.
- “Browse MCP servers from the CLI” documents `/mcp`.
- The configuration page names user/project files, precedence, and trust.
