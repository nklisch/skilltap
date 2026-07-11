---
source_handle: codex-build-plugins
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/build-plugins
provenance: source-direct
substrate_confidence: source-direct
---

# Build plugins

## Summary

OpenAI specifies the Codex plugin package, marketplace catalog, supported
source types, native marketplace commands, cache layout, and plugin-scoped
configuration. Each plugin requires `.codex-plugin/plugin.json`; skills,
hooks, app mappings, MCP configuration, and assets live at the plugin root.
Marketplace catalogs are JSON files with repo and personal default locations,
and entries may resolve local, Git-backed, or npm sources. Codex provides
native commands to add, list, upgrade, and remove tracked marketplaces.

## Key passages

### Authoring and native marketplace lifecycle (lines 707-776)

> "A marketplace is a JSON catalog of plugins." (Build your own curated plugin
> list, line 753)

- Local skills are recommended for one-repository or personal iteration;
  plugins are the distribution unit for stable shared workflows, bundled
  connectors or MCP configuration, and lifecycle hooks.
- `.codex-plugin/plugin.json` is the required plugin manifest.
- A local marketplace can be generated for testing an existing plugin.
- `codex plugin marketplace add` tracks GitHub shorthand, Git URLs, SSH URLs,
  or local marketplace roots; Git sources may be pinned with `--ref` and
  sparsely checked out with repeatable `--sparse` paths.
- Native `list`, `upgrade` (all or named), and `remove` marketplace commands
  inspect and manage configured sources.

### Marketplace locations and local installation (lines 751-859)

- A marketplace is a JSON plugin catalog; one catalog can expose one or many
  plugins.
- Default catalog locations are `$REPO_ROOT/.agents/plugins/marketplace.json`
  for repository scope and `~/.agents/plugins/marketplace.json` for personal
  scope.
- Marketplace-local plugin paths start with `./`, are relative to the
  marketplace root, and remain inside that root.
- The local examples place repository plugins under `$REPO_ROOT/plugins/` and
  personal plugins under `~/.codex/plugins/`, while noting those directories
  are conventions rather than fixed requirements.
- Updating a local plugin requires updating the directory addressed by the
  marketplace entry and restarting the desktop app.

### Marketplace metadata and remote sources (lines 875-964)

- Catalog entries include a stable name, source, installation policy,
  authentication policy, and category; a marketplace can also supply display
  metadata and ordering.
- Git-backed entries support repository-root and subdirectory forms plus
  `ref` or `sha` selectors. Failure to resolve one entry skips that entry rather
  than rejecting the whole marketplace.
- npm-backed entries accept a package, optional version/range/tag, and optional
  credential-free HTTPS registry URL. Codex uses npm's authentication config,
  requires the npm CLI, and downloads without lifecycle scripts.
- The desktop app reads the curated directory, repository `.agents` catalog,
  a legacy-compatible repository `.claude-plugin` catalog, and the personal
  `.agents` catalog.
- Installed plugins are copied to
  `~/.codex/plugins/cache/$MARKETPLACE_NAME/$PLUGIN_NAME/$VERSION/`; local
  installs use `local` as the version and run from the cached copy.
- Plugin enablement state is stored in `~/.codex/config.toml`.

### Package structure and policy (lines 966-1117)

- Only `plugin.json` belongs under `.codex-plugin/`; `skills/`, `hooks/`,
  `assets/`, `.mcp.json`, and `.app.json` remain at the plugin root.
- Manifest component paths are relative to the plugin root, begin with `./`,
  and remain inside the root.
- Plugin-scoped MCP policy can enable or disable a bundled server and configure
  its exposed tools and approval behavior without modifying the plugin.
- Plugin hooks require separate user review and trust even when the plugin is
  installed and enabled.
- Codex sets both Codex-native and Claude-compatible plugin root/data
  environment variables for hook processes.
