---
id: epic-expanded-harness-support-file-managed-opencode
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the OpenCode Adapter

## Checkpoint

Implement and register a distinct `OpenCodeAdapter` after exact native profile
validation. Use documented complete skill roots and regular layered config;
do not invoke the incomplete one-way plugin command or treat Bun's dependency
cache as a lifecycle API.

## Native contract

- Default executable: `opencode`.
- Complete skills: choose `.agents/skills` globally and in projects; observe
  OpenCode-owned and Claude-compatible roots as native precedence/unmanaged
  surfaces without rewriting them.
- MCP: global
  `${XDG_CONFIG_HOME:-~/.config}/opencode/opencode.json`, project
  `<project>/opencode.json`, editing only the `mcp` object.
- Status: bounded `opencode mcp list`; use `opencode mcp debug` only as a
  diagnostic next action.
- Secrets: OAuth tokens and startup cache remain outside inventory, state,
  findings, and writes.

## Implementation boundary

Add `crates/harnesses/src/adapters/opencode.rs` and
`opencode_managed.rs`. Keep `OpenCodeMcpCodec` private and explicit: transform
portable source MCP into OpenCode local/remote schema (native type, command
vector, environment, URL, headers, enabled state, and tool filtering) only when
all material semantics are attested. Never copy a source `mcpServers` object
wholesale. Shared code owns source reading, complete skill trees, manifests,
fingerprints, ownership, rollback, and lifecycle acceptance.

## Acceptance evidence

- Exact profile is mutable at global/project scope; malformed or unknown output
  cannot mutate.
- Complete canonical project skills need no redundant link.
- Local/remote fixtures encode exact OpenCode JSON and appear in version-pinned
  list status; project same-name values override global without deleting it.
- Unknown config fields, unrelated settings/plugins, and unowned MCP entries
  survive. Same-name unowned entries conflict.
- Ambiguous transports, literal secrets, OAuth-only behavior, and unattested
  fields are partial or blocked according to requiredness.
- Install/update/remove, target-local state, drift, rollback, pending recovery,
  and immediate-repeat no-op pass; `~/.cache/opencode` stays untouched.
