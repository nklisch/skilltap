---
source_handle: antigravity-customizations-1-1-13
fetched: 2026-08-15
source_path: /home/nathan/.gemini/antigravity-cli/builtin/skills/agy-customizations
provenance: source-direct
substrate_confidence: source-direct
---

# Antigravity customization contract in AGY 1.1.13

The installed `agy-customizations` skill documents Antigravity CLI's rules,
skills, plugins, hooks, MCP servers, discovery roots, and declared path
configuration. The source is bundled with the installed AGY 1.1.13 binary.

## Key passages

- `SKILL.md` names `.agents/` as the primary workspace customization root and
  `~/.gemini/config/` as the global customization root. It also lists `.agent/`,
  `_agents/`, and `_agent/` as workspace aliases.
- `docs/skills.md` defines a complete skill directory with top-level `SKILL.md`
  and required `name` and `description` frontmatter.
- `docs/plugins.md` defines `plugins/<name>/plugin.json` bundles containing
  skills, rules, hooks, and MCP configuration. It says `config.json` records
  plugin enablement separately from the installed plugin.
- `docs/mcp_servers.md` defines global
  `~/.gemini/config/mcp_config.json` and plugin-bundled `mcp_config.json`. It
  documents stdio servers with `command`, `args`, and `env`, plus SSE servers
  with `serverUrl`.
- `docs/hooks.md` defines synchronous command hooks for `PreToolUse`,
  `PostToolUse`, `PreInvocation`, `PostInvocation`, and `Stop`.
- `docs/json_configs.md` defines `skills.json` and `plugins.json` path entries,
  inheritance, inclusion, and exclusion filters.
- `SKILL.md` lists `.agents/rules/*.md` and triggered rule behavior, while
  `docs/rules.md` documents only hierarchical `AGENTS.md` and `GEMINI.md` files.
  This leaves standalone rule frontmatter behavior internally inconsistent.
