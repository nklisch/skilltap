---
source_handle: gemini-skills
fetched: 2026-07-12
source_url: https://geminicli.com/docs/cli/using-agent-skills/
provenance: source-direct
substrate_confidence: source-direct
---

# Gemini CLI skills

Gemini discovers skills from user `~/.gemini/skills` or `~/.agents/skills`, workspace `.gemini/skills` or `.agents/skills`, and extensions. A skill is a directory containing `SKILL.md` plus optional resources. Terminal utilities install, link, and uninstall skills, with `--scope workspace` for project scope.

## Key passages

- “Discovery tiers” lists user, workspace, extension, and `.agents/skills` aliases.
- “Install a skill” gives the default user scope and `--scope workspace`.
- “Uninstall a skill” removes an installed or linked skill by name.
