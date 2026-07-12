---
source_handle: qwen-extensions
fetched: 2026-07-12
source_url: https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/
provenance: source-direct
substrate_confidence: source-direct
---

# Qwen Code extensions

Qwen documents a native `qwen extensions` lifecycle with marketplace source add/list/update/remove, extension install/uninstall/enable/disable/update, and `--scope project` (workspace alias). Sources may be Claude marketplaces, Gemini extension repositories, npm, Git, local paths, or archives. Claude plugins are converted into Qwen manifests and skills; Gemini extensions are converted as well. Extensions can contain skills, agents, commands, MCP servers, and context files. Git/local installs are copied, and updates are explicit.

## Key passages

- “Managing marketplace sources” lists `qwen extensions sources add|list|update|remove`.
- “Choosing an install scope” distinguishes global/user and project/workspace.
- “Updating an extension” documents `qwen extensions update` and `--all`, with exact npm pins treated as current.
- “Custom skills” requires a `skills/<name>/SKILL.md` directory inside an extension.
- The Claude and Gemini sections describe native format conversion and preserved resources.
