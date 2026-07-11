---
source_handle: claude-settings
fetched: 2026-07-10
source_url: https://code.claude.com/docs/en/settings
provenance: source-direct
substrate_confidence: source-direct
---

# Claude Code settings

## Summary

Anthropic's settings reference defines managed, command-line, local, project, and user precedence, with distinct settings files for user, project, and local scopes. Plugin enablement is represented by `enabledPlugins` keys of the form `plugin-name@marketplace-name`; marketplace registration can be declared through `extraKnownMarketplaces`.

Repository settings do not silently install external plugins for collaborators. Trust and consent gates remain part of the native workflow. Marketplace policy can block or allow exact sources before network/filesystem operations, and native settings include per-marketplace automatic-update behavior.

## Anchored excerpts

**Plugin settings, line 731:**

> Controls which plugins are enabled.

**Extra known marketplaces, line 756:**

> Installation respects trust boundaries and requires explicit consent.

## Key passages and anchors

- **Configuration scopes, lines 103-153:** user settings live under `~/.claude/`, project settings under `.claude/`, and local settings at `.claude/settings.local.json`; precedence is managed, command line, local, project, user.
- **Plugin settings, lines 708-747:** `enabledPlugins` maps qualified plugin names to booleans across user, project, local, and managed settings; project and local precedence are described; an external source declaration is not itself installation for other users.
- **Extra known marketplaces, lines 750-805:** `extraKnownMarketplaces` declares sources; repository trust prompts precede marketplace/plugin installation; users can decline; supported source shapes and per-marketplace `autoUpdate` are documented.
- **Managed restrictions, lines 807-940:** `strictKnownMarketplaces` is managed-only, matches source specifications, and is enforced before network or filesystem work; `blockedMarketplaces` supplies the complementary deny policy.
- **Strict plugin-only customization, lines 980-982:** managed policy can block user/project skills, agents, hooks, and MCP servers so customization comes only from plugins or managed settings.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product reference
- Surface: Claude Code configuration and policy
- Retrieval depth: full page with targeted line reads
