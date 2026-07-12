---
source_handle: kilo-mcp
fetched: 2026-07-12
source_url: https://kilo.ai/docs/automate/mcp/using-in-kilo-code
provenance: source-direct
---

# Kilo Code MCP

Kilo stores MCP definitions in its main JSON/JSONC configuration. Global
configuration lives under `~/.config/kilo/kilo.jsonc`; project configuration
lives in project-root `kilo.jsonc` or `.kilo/kilo.jsonc`. Project entries take
precedence. The files are documented direct-edit surfaces, and Kilo detects
available tools and reports failed or authentication-required states.

## Key passages

- “Configuring MCP Servers” names both scopes and precedence.
- “Editing MCP Settings” says UI changes write the same files and direct editing
  is supported.
- “Config Format” defines local and remote schemas and enablement.
- “Troubleshooting” describes status evidence surfaced by Kilo.
