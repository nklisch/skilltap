---
source_handle: factory-mcp
fetched: 2026-07-12
source_url: https://docs.factory.ai/cli/configuration/mcp
provenance: source-direct
---

# Factory Droid MCP

Factory documents MCP server configuration as supported JSON files at user and
project scope. User servers live in `~/.factory/mcp.json`; repository servers
live in `.factory/mcp.json`. User definitions take precedence when names
collide. Project definitions are removed by editing the project file, while the
CLI manages user entries. Droid reloads configuration changes automatically.

## Key passages

- The configuration table names both supported paths and their scope.
- “How Layering Works” states user definitions override project definitions.
- “Removing Servers” distinguishes user CLI removal from direct project-file editing.
- “Configuration Schema” defines `mcpServers`, stdio/http fields, disablement,
  and automatic reload after file changes.
