---
source_handle: codex-skills
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/build-skills
provenance: source-direct
substrate_confidence: source-direct
---

# Build skills

## Summary

OpenAI defines a skill as a directory containing required `SKILL.md` plus
optional scripts, references, assets, and Codex metadata. `SKILL.md` requires
`name` and `description`. Codex discovers repository, user, administrator, and
system skills, supports symlinked directories, automatically detects local
changes, and permits per-skill disabling through user configuration.

## Key passages

### Shape and activation (lines 716-759)

> "A skill is a directory with a `SKILL.md` file plus optional scripts and
> references." (Build skills, line 720)

- Codex initially exposes skill name, description, and file path, loading full
  `SKILL.md` only when the skill is selected.
- A skill is the complete directory containing `SKILL.md`, with optional
  `scripts/`, `references/`, `assets/`, and `agents/openai.yaml`.
- `SKILL.md` must include `name` and `description`.
- Skills may be invoked explicitly or selected implicitly from their
  descriptions.
- Codex automatically detects skill changes; restart is the documented remedy
  when an update does not appear.

### Discovery scopes (lines 760-777)

- In repositories, Codex scans `.agents/skills` directories from the current
  working directory upward to the repository root.
- User skills live at `$HOME/.agents/skills`; administrator skills at
  `/etc/codex/skills`; system skills are bundled by OpenAI.
- Same-named skills are not merged and may both appear in selectors.
- Symlinked skill folders are supported and their targets are followed.
- Plugins, rather than local skill locations, are the recommended distribution
  unit for reusable skills beyond one repository or when bundling connectors.

### Enablement (lines 792-800)

- `[[skills.config]]` in `~/.codex/config.toml` can disable a skill by its
  `SKILL.md` path without deleting the directory.
- Codex should be restarted after changing this configuration.
