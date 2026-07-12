---
source_handle: junie-extensions
fetched: 2026-07-12
source_url: https://junie.jetbrains.com/docs/junie-cli-extensions.html
provenance: source-direct
substrate_confidence: source-direct
---

# Junie CLI extensions

Junie supports native and Claude marketplace manifests, extension skills, user/project scopes, and update/remove operations. However, current documentation exposes management through the interactive `/extensions` screen and slash commands, while extension content is kept in `~/.junie/extensions/` caches. The documented state files are inspectable, but the lifecycle execution surface is not a deterministic shell CLI.

## Key passages

- The marketplace section lists git, local, URL, native Junie, and Claude manifest sources.
- The install section stores project/user references in `extensions.json` and content in a user cache.
- The remove/update sections describe Installed-tab actions rather than non-interactive shell commands.
