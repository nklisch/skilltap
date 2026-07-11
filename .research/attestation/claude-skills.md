---
source_handle: claude-skills
fetched: 2026-07-10
source_url: https://code.claude.com/docs/en/slash-commands
provenance: source-direct
substrate_confidence: source-direct
---

# Claude Code skills

## Summary

Anthropic's skills guide treats a skill as a directory whose required entry point is `SKILL.md`; supporting files in that directory are optional and are part of the skill's usable material. Personal skills live at `~/.claude/skills/<name>/SKILL.md`, project skills at `.claude/skills/<name>/SKILL.md`, enterprise skills in managed settings, and plugin skills under `<plugin>/skills/<name>/SKILL.md`.

The guide documents precedence and namespacing, symlink support for personal/project skill entries, live change detection, nested project skills, and Claude-specific frontmatter extensions. YAML frontmatter fields are optional, with `description` recommended; several Claude-only fields alter invocation, tools, model, subagent context, and hooks.

## Anchored excerpts

**Create your first skill, line 156:**

> Every skill needs a `SKILL.md` file with two parts.

**Supporting files, line 220:**

> The `SKILL.md` contains the main instructions and is required.

## Key passages and anchors

- **Create your first skill, lines 142-168:** a personal skill is a directory under `~/.claude/skills`; `SKILL.md` supplies YAML frontmatter and Markdown instructions.
- **Where skills live, lines 184-211:** enterprise, personal, project, and plugin paths and precedence are defined; plugin skills are namespaced; personal/project skill entries may be directory symlinks; nested project skills are discovered contextually.
- **Live change detection, lines 204-207:** edits to skill text are watched in current sessions, while non-skill plugin components require `/reload-plugins`.
- **Supporting files, line 220 and supporting-files section:** `SKILL.md` is required and templates, examples, scripts, and references may accompany it in the skill directory.
- **Frontmatter reference, lines 264-292:** all fields are optional and `description` is recommended; Claude-specific fields include `when_to_use`, invocation controls, tool restrictions, model/effort, forked context, agents, and hooks.
- **Command-name derivation, lines 297 onward:** ordinary skill invocation names derive primarily from directory placement; plugin-root `SKILL.md` is a documented special case.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product guide and field reference
- Surface: Claude Code skills
- Retrieval depth: full page with targeted line reads
