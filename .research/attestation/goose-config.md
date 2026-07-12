---
source_handle: goose-config
fetched: 2026-07-12
source_url: https://goose-docs.ai/docs/guides/config-files/
provenance: source-direct
---

# Goose global extension configuration

Goose stores its persistent provider, settings, and extension configuration in
one user-level YAML file: `~/.config/goose/config.yaml` on macOS/Linux. MCP
servers are represented as extensions under the `extensions` key. The page
does not document a corresponding automatically loaded project configuration
file for extensions.

## Key passages

- “Configuration Overview” names the primary user-level file.
- “Extensions” defines stdio extension command, arguments, environment, tools,
  timeout, and enablement.
- The page's configuration-file inventory does not identify a project extension file.
