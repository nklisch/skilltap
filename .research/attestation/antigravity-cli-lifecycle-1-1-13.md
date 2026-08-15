---
source_handle: antigravity-cli-lifecycle-1-1-13
fetched: 2026-08-15
source_path: /home/nathan/.local/bin/agy
provenance: source-direct
substrate_confidence: source-direct
---

# Antigravity CLI 1.1.13 lifecycle behavior

The installed `agy` binary was exercised against a temporary isolated home with
a synthetic local plugin. No user configuration, authentication, project, or
network resource participated.

## Key passages

- `agy --version` emits `1.1.13`.
- `agy plugin --help` advertises install, uninstall, list, enable, disable,
  validate, import, and marketplace-link commands. It does not advertise a
  plugin update command or an output-format option.
- `agy plugin validate <directory>` validates component structure and attempts
  to resolve configured MCP executables from `PATH`.
- `agy plugin install <directory>` copies a valid plugin into
  `~/.gemini/config/plugins/<name>/` and writes
  `~/.gemini/config/import_manifest.json`.
- `agy plugin list` emits a JSON object with an `imports` array when a plugin is
  installed. With no imports it emits the human text `No imported plugins.`.
- `agy plugin disable <name>` writes
  `~/.gemini/config/config.json` under `plugins.<name>.enabled`. Enable changes
  the same native setting.
- `agy plugin uninstall <name>` removes the imported plugin and its manifest
  record. It retains the now-empty configuration files and plugin directory.
- The observed lifecycle is global. The command help and installed documents do
  not establish project-scoped native installation, marketplace identity,
  installed revision, or plugin update semantics.
