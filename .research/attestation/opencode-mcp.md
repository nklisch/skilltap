---
source_handle: opencode-mcp
fetched: 2026-07-14
source_urls:
  - https://dev.opencode.ai/docs/mcp-servers/
  - https://dev.opencode.ai/docs/config/
  - https://opencode.ai/config.json
provenance: source-direct-plus-isolated-execution
substrate_confidence: source-direct-and-runtime
---

# OpenCode MCP and configuration

OpenCode stores MCP declarations in the `mcp` object of regular JSON or JSONC
configuration. The documented global file is
`~/.config/opencode/opencode.json`; an isolated runtime confirmed that
`XDG_CONFIG_HOME/opencode/opencode.json` is the corresponding relocatable path.
The project file is `opencode.json` at the project root.

Configuration layers are merged rather than replaced. Remote organizational
configuration is loaded first, followed by global configuration, optional
`OPENCODE_CONFIG`, project configuration, `.opencode` directories, inline
`OPENCODE_CONFIG_CONTENT`, and managed settings. Project values override
conflicting global values while unrelated global declarations remain effective.

The current schema defines exactly two MCP server forms:

- Local: `type: "local"`, a string-array `command`, optional `cwd`,
  `environment`, `enabled`, and positive-integer `timeout`.
- Remote: `type: "remote"`, a `url`, optional `enabled`, `headers`, `oauth`, and
  positive-integer `timeout`.

OpenCode's tool filtering is a top-level `tools` permission map using server
name patterns; it is not a property of the `mcp` server object. A converter
that cannot edit that separate top-level policy must reject source tool-filter
fields instead of silently dropping them.

## Effective status grammar

The documented non-interactive probe is `opencode mcp list`; `opencode mcp debug
<name>` is a diagnostic for a failing or authenticating server. In the
validated 1.18.1 runtime, list output is ANSI-decorated human text with lines
of the form `●  ○ <name> disabled`, `●  ✓ <name> connected`, or `●  ✗ <name>
failed`, followed by `└  N server(s)`. Empty state is `No MCP servers configured`.
The adapter strips ANSI decoration but fails closed on malformed table shape or
count.

OAuth tokens are stored separately by OpenCode at
`~/.local/share/opencode/mcp-auth.json`. They are not part of the managed MCP
document and never enter skilltap state, inventory, findings, or materialized
files.
