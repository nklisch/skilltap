---
source_handle: copilot-skills
fetched: 2026-07-12
source_url: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills
provenance: source-direct
substrate_confidence: source-direct
---

# GitHub Copilot CLI skills

Copilot defines skills as directories of instructions, scripts, and resources with a required `SKILL.md`. Project roots include `.github/skills`, `.claude/skills`, and `.agents/skills`; personal roots include `~/.copilot/skills` and `~/.agents/skills`. The shell exposes `copilot skill list|add|remove`, while plugin-provided skills are managed through their plugin.

## Key passages

- “Adding a skill that someone else has created” requires a directory containing `SKILL.md` and optional files.
- The project/personal path list gives both scopes and the `.agents/skills` interoperability path.
- The CLI commands section lists `copilot skill list`, `add`, and `remove`.
