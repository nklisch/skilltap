---
source_handle: junie-skills
fetched: 2026-07-12
source_url: https://junie.jetbrains.com/docs/agent-skills.html
provenance: source-direct
---

# Junie Agent Skills

Junie defines a skill as a folder containing `SKILL.md` plus instructions,
templates, scripts, and reference material. Junie CLI scans project skills at
`<project>/.junie/skills/<name>/` and user skills at
`~/.junie/skills/<name>/`. Supporting subdirectories are explicitly supported,
and the documentation shows whole-directory copy installation.

## Key passages

- “Agent skills are folders” defines the resource boundary.
- “Skill location” names project and user roots.
- “Use subdirectories for complex skills” lists checklists, scripts, and templates.
- “Adding a skill” copies an entire skill folder into either scope.
