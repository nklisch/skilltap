---
source_handle: kimi-plugins
fetched: 2026-07-12
source_url: https://www.kimi.com/code/docs/en/kimi-code-cli/customization/plugins
provenance: source-direct
substrate_confidence: source-direct
---

# Kimi Code CLI plugins

Kimi Code provides marketplace plugins, plugin skills, GitHub/local installs, enable/disable/remove/list slash commands, and commit-addressable sources. Current docs say plugins are installed per-user and apply to all projects; project scope is not supported. Routine lifecycle is driven by the interactive TUI or slash commands, and local installs are copied to a managed directory.

## Key passages

- The installation table is `/plugins` and slash-command based.
- The notes explicitly state project-level installation is not supported.
- Managed-copy behavior means editing the source directory does not update the installed plugin.
