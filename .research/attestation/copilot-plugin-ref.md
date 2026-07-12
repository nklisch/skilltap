---
source_handle: copilot-plugin-ref
fetched: 2026-07-12
source_url: https://docs.github.com/en/enterprise-cloud%40latest/copilot/reference/copilot-cli-reference/cli-plugin-reference
provenance: source-direct
substrate_confidence: source-direct
---

# GitHub Copilot CLI plugin reference

Copilot CLI exposes native plugin install/uninstall/list/update/enable/disable plus marketplace add/list/browse/remove. Install sources include registered marketplaces, GitHub, arbitrary Git URLs, and local paths. Plugins require a root `plugin.json`; skills default to `skills/`. Installed plugins and marketplace caches have documented locations and `COPILOT_CACHE_HOME` override.

## Key passages

- The CLI commands table lists the full plugin and marketplace lifecycle.
- Marketplace manifests are accepted in `.github/plugin/` and `.claude-plugin/`.
- File locations distinguish installed plugins from marketplace caches.
