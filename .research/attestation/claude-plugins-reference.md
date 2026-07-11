---
source_handle: claude-plugins-reference
fetched: 2026-07-10
source_url: https://code.claude.com/docs/en/plugins-reference
provenance: source-direct
substrate_confidence: source-direct
---

# Claude Code plugins reference

## Summary

Anthropic's technical reference defines a plugin as a self-contained directory whose components can include skills, agents, hooks, MCP servers, LSP servers, monitors, output styles, themes, executables, and limited default settings. The manifest at `.claude-plugin/plugin.json` is optional; when present, `name` is its sole required field. Default component directories live at the plugin root, not under `.claude-plugin/`.

Marketplace-installed plugins are copied into a versioned user cache. A plugin may instead load directly from a skills directory when its folder contains `.claude-plugin/plugin.json`; that path has different install and trust behavior. User, project, local, and managed plugin scopes map to distinct settings surfaces.

The reference also specifies non-interactive plugin lifecycle commands and version resolution. Update detection uses the plugin manifest version, marketplace-entry version, or git commit SHA in that order.

## Anchored excerpts

**Overview, line 106:**

> A plugin is a self-contained directory of components that extends Claude Code with custom functionality.

**Metadata fields, line 461:**

> If omitted, Claude Code falls back to the git commit SHA.

## Key passages and anchors

- **Plugin components reference, lines 106-128:** plugins are self-contained; skills use complete directories containing `SKILL.md`, may include supporting files, and may alternatively use a root `SKILL.md` for a single-skill plugin.
- **Plugin installation scopes, lines 359-368:** user scope maps to `~/.claude/settings.json`, project to `.claude/settings.json`, local to `.claude/settings.local.json`, and managed to read-only managed settings.
- **Skills-directory plugins, lines 373-398:** a folder under a skills directory with `.claude-plugin/plugin.json` loads as `<name>@skills-dir` without marketplace installation; personal and project locations have distinct trust and discovery behavior.
- **Plugin manifest schema, lines 404-467:** the manifest is optional; `name` is the only required field when it exists; supported component-path and metadata fields are enumerated; unknown top-level fields are warnings rather than runtime failures.
- **Plugin caching and file resolution, lines 639-658:** marketplace installs are copied into `~/.claude/plugins/cache`; every installed version gets a separate cache directory; paths cannot escape the plugin root; symlinks have documented copy/preservation behavior.
- **Standard plugin layout, lines 668-728:** components are rooted beside `.claude-plugin/`; only `plugin.json` belongs inside `.claude-plugin/`; a plugin-root `CLAUDE.md` is not loaded as project context.
- **CLI commands reference, lines 732-900:** `claude plugin install`, `uninstall`, `enable`, `disable`, `update`, and `list` provide native lifecycle operations; install/update/remove accept explicit scope; list supports JSON.
- **Version management, lines 1053-1068:** update detection keys on resolved version in priority order: plugin manifest version, marketplace entry version, then git commit SHA; omitting explicit versions makes new git commits new versions.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product reference
- Surface: Claude Code plugins
- Retrieval depth: full page with targeted line reads
