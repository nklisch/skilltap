---
source_handle: kiro-skills
fetched: 2026-07-12
source_url: https://kiro.dev/docs/cli/skills/
provenance: source-direct
---

# Kiro CLI Agent Skills

Kiro CLI implements the Agent Skills format. Each skill is a directory with a
required `SKILL.md` and optional reference files. Workspace skills live in
`.kiro/skills`; global skills in `~/.kiro/skills`; workspace definitions take
priority. The default agent loads both locations without extra configuration.

## Key passages

- “Skill locations” defines global and workspace roots and precedence.
- “Creating a skill” shows a directory with `SKILL.md` and `references/`.
- “Default agent” states both locations load automatically.
- `/context show` is named as a way to inspect available skills.
