---
source_handle: pi-settings
fetched: 2026-07-12
source_url: https://pi.dev/docs/latest/settings
provenance: source-direct
substrate_confidence: source-direct
---

# Pi settings

Pi uses `~/.pi/agent/settings.json` for global settings and `.pi/settings.json` for project overrides. Project settings override global settings; trust controls whether project resources and packages are loaded. Package directories and settings paths are inspectable and configurable.

## Key passages

- The settings location table distinguishes global and project scopes.
- Project trust controls loading and package installation.
- Resource settings expose `packages`, `skills`, and other paths.
