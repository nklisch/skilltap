---
source_handle: opencode-plugins
fetched: 2026-07-12
source_url: https://dev.opencode.ai/docs/plugins/
provenance: source-direct
substrate_confidence: source-direct
---

# OpenCode plugins

OpenCode loads local plugins from project `.opencode/plugins/` and global `~/.config/opencode/plugins/`, or npm package names from `opencode.json`. Bun installs npm plugins at startup and caches dependencies under `~/.cache/opencode/node_modules/`. The docs describe a config/startup model rather than a marketplace or full package lifecycle.

## Key passages

- “Use a plugin” lists project/global local paths and npm config.
- “How plugins are installed” says Bun installs at startup and identifies the cache.
