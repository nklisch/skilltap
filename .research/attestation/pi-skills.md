---
source_handle: pi-skills
fetched: 2026-07-12
source_url: https://pi.dev/docs/latest/skills
provenance: source-direct
substrate_confidence: source-direct
---

# Pi skills

Pi implements Agent Skills and requires a skill directory with `SKILL.md`, allowing supporting files. It discovers global/project `.agents/skills` and `.pi/skills`, and can explicitly load Claude and Codex skill directories through settings. It also discovers `AGENTS.md`/`CLAUDE.md` context files.

## Key passages

- “Locations” lists global and project `.agents/skills` and package skill roots.
- “Using Skills from Other Harnesses” gives Claude and Codex directory settings.
- “Skill Structure” requires a directory containing `SKILL.md` and permits siblings.
