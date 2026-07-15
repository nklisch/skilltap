---
id: epic-expanded-harness-support-file-managed-opencode
kind: story
stage: done
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

## Implementation

Registered `OpenCodeAdapter` only for the exact validated profile
`opencode-1-18-1` / OpenCode `1.18.1`. The adapter uses the shared scope-aware
managed lifecycle for global and project scopes, keeps `.agents/skills` as the
canonical destination, observes OpenCode/Claude-compatible skill roots without
rewriting them, and edits only the selected `mcp` object in the documented
configuration file.

`OpenCodeMcpCodec` explicitly converts source local and remote MCP values into
OpenCode's native `type`, command vector, environment, URL, headers, enabled,
and timeout fields. It rejects ambiguous transports, source-root-relative
commands, literal environment/header values, OAuth configuration objects, and
tool-filter fields because current OpenCode places tool filtering in the
separate top-level `tools` policy rather than inside `mcp`; silently dropping
that policy would not be faithful. `oauth: false` is preserved. OAuth tokens,
OpenCode data, Bun/npm packages, plugin directories, and `~/.cache/opencode`
are never lifecycle surfaces.

The version-pinned `opencode mcp list` decoder accepts the observed ANSI table
grammar and empty-state message, validates server markers and the reported
count, and fails closed on malformed or adjacent output. `opencode mcp debug`
is not invoked automatically.

## Current evidence and provenance

Official current sources were refreshed on 2026-07-14:

- Skills: <https://dev.opencode.ai/docs/skills/> — complete skill directories
  and the six OpenCode, Claude-compatible, and `.agents/skills` roots.
- Configuration/MCP: <https://dev.opencode.ai/docs/config/>,
  <https://dev.opencode.ai/docs/mcp-servers/>, and
  <https://opencode.ai/config.json> — layered global/project JSON/JSONC, MCP
  schema, precedence, and top-level tool policy.
- CLI/plugins: <https://dev.opencode.ai/docs/cli/> and
  <https://dev.opencode.ai/docs/plugins/> — `mcp list`/`debug`, the incomplete
  one-way plugin command, and the Bun cache boundary.
- Runtime/package evidence: `.research/attestation/opencode-version.md` records
  the official `anomalyco/opencode` `v1.18.1` release, npm package provenance,
  registry signature key, published integrity values, acquired Linux x64
  SHA-512/SHA-256, and exact `--version` bytes (`1.18.1\\n`). The binary was
  run under isolated HOME/XDG/data/cache/project roots.

The isolated runtime confirmed global
`$XDG_CONFIG_HOME/opencode/opencode.json`, project `opencode.json`, merged
configuration, and project same-name MCP override without deleting the global
value. Native OpenCode itself creates runtime data/model/package cache while
running; that behavior is explicitly not adopted as a skilltap write API.

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

## Verification

- `cargo test -p skilltap-harnesses --all-targets` — passed.
- Focused adapter tests cover exact version/profile gating, ANSI `mcp list`
  grammar, global/project probe arguments, empty/malformed status, local/remote
  codec mapping, secret/OAuth/filter rejection, and unknown-field preservation.
- Compiled CLI regression covers global/project install, complete skills,
  OpenCode MCP conversion, preserved config/plugin fields, cache sentinel
  stability, and zero target/state writes for adjacent versions `1.18.0` and
  `1.18.2`.
- `cargo test --workspace --all-targets` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.
