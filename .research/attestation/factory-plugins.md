---
source_handle: factory-plugins
fetched: 2026-07-12
source_url: https://docs.factory.ai/cli/configuration/plugins
provenance: source-direct
substrate_confidence: source-direct
---

# Factory Droid plugins

Factory documents a native `droid plugin` CLI for marketplace add/remove/list/update and plugin install/uninstall/update/list. Commands accept `--scope user|project`. Plugins bundle skills, commands, agents, hooks, and MCP servers; Claude Code plugin format is documented as interoperable. Plugin updates resolve the latest marketplace Git commit rather than a semantic version, and version pinning is not supported. The CLI keeps installed plugin content in a cache, so skilltap must use the native lifecycle instead of editing cache files.

## Key passages

- “CLI commands (for scripting)” lists marketplace and plugin lifecycle commands and explicit user/project scope.
- “Version tracking” states updates fetch the latest marketplace commit and do not support pinning.
- “Claude Code compatibility” says Claude plugins can be installed directly.
- “Removing a marketplace” notes installed plugins may continue to work from cache.
