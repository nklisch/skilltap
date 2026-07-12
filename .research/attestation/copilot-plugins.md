---
source_handle: copilot-plugins
fetched: 2026-07-12
source_url: https://docs.github.com/en/copilot/concepts/agents/about-plugins
provenance: source-direct
substrate_confidence: source-direct
---

# GitHub Copilot plugins

GitHub describes plugins as packages containing any combination of agents, skills, hooks, MCP, and LSP. A marketplace is a versioned registry described by `marketplace.json`; Copilot CLI can install imperatively or through declarative `enabledPlugins` in user or repository settings. The documentation lists Claude marketplaces as examples and supports local, Git, and GitHub sources.

## Key passages

- The plugin structure example shows `skills/<name>/SKILL.md` alongside the root manifest.
- The marketplace section defines registries and versioned entries.
- The scope section names user `~/.copilot/settings.json` and repository `.github/copilot/settings.json`.
