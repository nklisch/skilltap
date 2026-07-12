---
source_handle: amp-manual
fetched: 2026-07-12
source_url: https://ampcode.com/manual
provenance: source-direct
---

# Amp skills and MCP

Amp defines skills as directories containing `SKILL.md` plus bundled resources.
Project skills load from `.agents/skills`; user skills load from
`~/.config/agents/skills`, `~/.agents/skills`, and Amp-specific roots. MCP
servers are configured under `amp.mcpServers` in user or nearest workspace
settings; workspace definitions override user definitions and require explicit
trust. `amp mcp doctor` and CLI MCP commands expose state.

## Key passages

- “Agent Skills” defines complete directories and user/project paths.
- “MCP Servers in Skills” permits a skill-local `mcp.json` and tool filtering.
- “Configuration” names user settings and nearest `.amp/settings.json` workspace settings.
- “Workspace MCP Server Trust” documents approval and `amp mcp doctor` evidence.
