---
source_handle: codex-config-reference
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/config-file/config-reference
provenance: source-direct
substrate_confidence: source-direct
---

# Configuration reference

## Summary

The Codex configuration reference specifies user-configurable plugin-scoped MCP
policy and administrator requirements for allowed marketplace sources and
plugin-bundled MCP identities. Marketplace source restrictions can match exact
Git repositories, Git hosts, or local directories and apply to add, install,
and Git refresh operations when enabled.

## Key passages

### Plugin-scoped MCP configuration (lines 1487-1502)

- `plugins.<plugin>.mcp_servers.<server>.enabled` controls a bundled server
  without changing the plugin manifest.
- Server policy can restrict enabled and disabled tools and set default or
  per-tool approval behavior.

### Marketplace requirements (lines 4940-4968)

> "Admin requirements for plugin marketplace sources." (Requirements reference,
> line 4940)

- Administrators may define named allowed marketplace source rules using exact
  Git repositories, host patterns, or normalized absolute local paths.
- Exact Git rules may optionally constrain the ref.
- With `marketplaces.restrict_to_allowed_sources = true`, restrictions apply
  when users add marketplaces, install plugins, and refresh configured Git
  marketplaces.
- Codex-managed OpenAI marketplaces remain allowed when their reserved name and
  source match; the restriction does not remove already configured sources from
  runtime consideration.

### Plugin server requirements (lines 5042-5087)

- Administrators can require exact identities for MCP servers bundled by a
  particular plugin.
- When the plugin requirements table is present, unlisted plugin/server pairs
  are disabled.
- Stdio identity requirements can match executable and ordered arguments;
  remote identities can match URL exactly, by prefix, or by regular expression.
