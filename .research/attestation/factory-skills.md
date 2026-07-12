---
source_handle: factory-skills
fetched: 2026-07-12
source_url: https://docs.factory.ai/cli/configuration/skills
provenance: source-direct
substrate_confidence: source-direct
---

# Factory Droid skills

Factory defines a skill as a directory containing `SKILL.md` or `skill.mdx`, with optional supporting files. Workspace skills live under `<repo>/.factory/skills/`, personal skills under `~/.factory/skills/`, and a compatibility path under `.agent/skills/`. Skills can be user-invoked or model-invoked, controlled by frontmatter.

## Key passages

- The “What is a skill?” section requires a skill directory and permits supporting files.
- The “Where skills live” table distinguishes workspace and personal scopes.
- Frontmatter supports `name`, `description`, `user-invocable`, and `disable-model-invocation`.
