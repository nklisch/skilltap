---
source_handle: opencode-skills
fetched: 2026-07-14
source_url: https://dev.opencode.ai/docs/skills/
provenance: source-direct
substrate_confidence: source-direct
---

# OpenCode skills

The current official skills contract manages a complete directory, not only
`SKILL.md`. OpenCode searches the following global and project roots:

- Global `~/.config/opencode/skills/<name>/SKILL.md`.
- Global `~/.claude/skills/<name>/SKILL.md`.
- Global `~/.agents/skills/<name>/SKILL.md`.
- Project `.opencode/skills/<name>/SKILL.md`.
- Project `.claude/skills/<name>/SKILL.md`.
- Project `.agents/skills/<name>/SKILL.md`.

For project-local paths OpenCode walks upward to the Git worktree. The
frontmatter requires `name` and `description`; `license`, `compatibility`, and
string-to-string `metadata` are recognized, while unknown frontmatter fields
are ignored. Names are 1–64 lowercase alphanumeric segments separated by
single hyphens and must match the containing directory. Descriptions are
1–1024 characters.

## Key passages

- The official location table lists all six roots above.
- The format section requires YAML frontmatter and a complete directory with
  top-level `SKILL.md`.
- The page states that sibling resources remain part of the skill and that
  skill loading is on demand.
