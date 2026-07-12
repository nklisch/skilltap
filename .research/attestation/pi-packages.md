---
source_handle: pi-packages
fetched: 2026-07-12
source_url: https://pi.dev/docs/latest/packages
provenance: source-direct
substrate_confidence: source-direct
---

# Pi packages

Pi packages bundle extensions, skills, prompt templates, and themes. Native commands install, remove, list, and update packages from npm, Git, URLs, or local paths. Global installs write `~/.pi/agent/settings.json`; project installs use `.pi/settings.json`. Git refs may be pinned and reconciled; exact ref changes are explicit.

## Key passages

- “Install and Manage” lists `pi install`, `remove`, `list`, `update --all`, and per-package update.
- “Package Sources” defines npm, Git, and local paths plus global/project checkout roots.
- “Scope and Deduplication” defines global/project precedence.
