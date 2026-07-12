---
source_handle: copilot-mcp
fetched: 2026-07-12
source_url: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
provenance: source-direct
---

# GitHub Copilot CLI MCP

Copilot CLI supports user MCP definitions in `~/.copilot/mcp-config.json` and
workspace/repository definitions in `.mcp.json` or `.github/mcp.json`. The
shell exposes `copilot mcp add`, `list`, `get`, and `remove`; list/get accept
JSON output. Effective servers are merged from user, workspace, and plugin
sources, with workspace sources taking precedence over user definitions.

## Key passages

- “Using the `copilot mcp add` subcommand” documents non-interactive user setup.
- “Editing the configuration file” defines the user JSON schema.
- “Managing MCP servers” documents list/get/remove and JSON output.
- The loading-priority section names workspace `.mcp.json` and
  `.github/mcp.json` above user configuration.
