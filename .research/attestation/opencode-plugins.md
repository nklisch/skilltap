---
source_handle: opencode-plugins
fetched: 2026-07-14
source_url: https://dev.opencode.ai/docs/plugins/
provenance: source-direct-plus-isolated-execution
substrate_confidence: source-direct-and-runtime
---

# OpenCode plugins

OpenCode loads local plugins from project `.opencode/plugins/` and global
`~/.config/opencode/plugins/`, or npm package names from the `plugin` array in
`opencode.json`. npm plugins are installed automatically by Bun at startup and
cached under `~/.cache/opencode/node_modules/`; local plugins are loaded from
their directories.

This is a configuration/startup model, not a deterministic marketplace or
complete native plugin lifecycle. skilltap therefore never invokes the
one-way plugin command, edits npm plugin configuration as a lifecycle shortcut,
or writes the Bun/OpenCode caches. The adapter observes plugin paths as native
unmanaged surfaces and owns only its `.agents/skills` and `mcp` projections.

## Key passages

- The official plugin location table lists global and project local plugin
  directories.
- The installation section says Bun installs npm plugins at startup and names
  the cache path.
- The load-order section places global config before project config and then
  global/project plugin directories.
