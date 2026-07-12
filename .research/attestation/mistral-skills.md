---
source_handle: mistral-skills
fetched: 2026-07-12
source_url: https://docs.mistral.ai/vibe/code/cli/skills
provenance: source-direct
substrate_confidence: source-direct
---

# Mistral Vibe skills

Vibe supports Agent Skills directories with `SKILL.md`, `.vibe/skills` project paths, `~/.vibe/skills` user paths, `.agents/skills` compatibility, and enable/disable filters in `config.toml`. The documentation does not provide marketplace registration or install/update/remove lifecycle commands for skills or plugins.

## Key passages

- “Skill locations” lists project, user, and `.agents/skills` paths.
- The configuration example uses `skill_paths`, `enabled_skills`, and `disabled_skills` rather than a package manager.
