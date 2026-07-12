---
source_handle: gemini-extensions
fetched: 2026-07-12
source_url: https://github.com/google-gemini/gemini-cli/blob/main/docs/extensions/reference.md
provenance: source-direct
substrate_confidence: source-direct
---

# Gemini CLI extensions

Gemini CLI exposes native extension install, uninstall, enable, disable, update, list, link, and validate commands. Install accepts a GitHub URL or local path and optional ref; extensions are copied into the Gemini extension directory. Extensions can bundle `skills/<name>/SKILL.md`, context files, commands, hooks, MCP servers, and subagents. Enable/disable explicitly supports user/workspace scope.

## Key passages

- The command reference lists install/update/uninstall/enable/disable and their flags.
- The extension format section requires `gemini-extension.json` and defines bundled skills.
- The install section says the CLI copies the extension and requires explicit update to pull source changes.
