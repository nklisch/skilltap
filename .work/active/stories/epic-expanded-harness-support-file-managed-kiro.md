---
id: epic-expanded-harness-support-file-managed-kiro
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/kiro-cli-2.12.2.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Kiro CLI Adapter

## Checkpoint

Implement and register a distinct `KiroAdapter` after exact native profile
validation. Target Kiro CLI Agent Skills and MCP files only; do not translate
or manage the separate IDE Power package lifecycle.

## Native contract

- Default executable: `kiro-cli`; global home is
  `${KIRO_HOME:-~/.kiro}` through validated `PlatformPaths`.
- Complete skills: `<kiro-home>/skills` globally and
  `<project>/.kiro/skills` in projects, with project precedence.
- MCP: `<kiro-home>/settings/mcp.json` and
  `<project>/.kiro/settings/mcp.json`, editing only `mcpServers`.
- Reload/status: documented file hot reload followed by bounded
  `kiro-cli mcp list`; `/mcp` remains interactive diagnostics.

## Implementation boundary

Add `crates/harnesses/src/adapters/kiro.rs` and `kiro_managed.rs`.
`KiroSkillProjection` supplies the distinct project root so the completed
project-skill lifecycle creates/repairs/removes a relative per-skill link to the
canonical `.agents/skills` tree; the adapter contains no symlink code. Managed
plugin components remain plugin-owned complete trees under Kiro's native root
and use shared ownership/rollback. Preserve disabled state, tool filters,
unrelated document fields, and unowned servers.

## Acceptance evidence

- Exact profile and `kiro-cli` default binary work in both scopes; unknown
  versions are observe-only.
- Default and overridden `KIRO_HOME` derive exact roots without moving canonical
  global instructions or another harness's state.
- Project standalone install produces the expected shared-contract relative
  link and inherits no-follow drift/repair/removal behavior.
- Plugin skill+MCP install, workspace-over-global precedence, hot reload
  observation, update, removal, rollback, pending recovery, target isolation,
  and immediate repeat pass.
- Disabled/tool-filter semantics survive round trip.
- Power-required sources block; optional Power/hook/steering loss is explicit
  and acknowledgment-gated. IDE/Power caches remain untouched.

## Implementation evidence

- Added private provisional `kiro.rs` and `kiro_managed.rs` modules. The exact
  `kiro-cli 2.12.2` profile, default binary, global/project paths, explicit
  `mcp list global|workspace` argv, hot-reload declaration, complete skill-tree
  projection, Kiro MCP codec, disabled/tool-filter preservation, ownership and
  drift checks, Power blocking, and optional unsupported-component acknowledgment
  paths are implemented against the shared ports.
- Added current source-direct evidence in
  `.research/attestation/kiro-cli-2.12.2.md`, including the official stable
  manifest, exact artifact SHA-256, isolated version output, current docs, and
  the effective-observation limitation.
- Added focused profile/version/probe/codec tests and explicit KIRO_HOME
  default/override coverage in core runtime tests.
- The modules are intentionally test-only and absent from `TargetRegistry::canonical()`
  and public exports. This ensures the current target has zero CLI writes/state
  while the evidence gate is unresolved; no OpenCode/Gemini files, caches,
  Powers, IDE state, `.pi/`, or `.work/bin/work-view` were changed.

## Verification

- `cargo test --workspace --all-targets` — 621 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.

## Blocker

The official stable manifest and isolated runtime establish Kiro CLI `2.12.2`,
its checksum, both skill/MCP scopes, precedence, schema fields, and hot reload.
However, both isolated `kiro-cli mcp list global` and
`kiro-cli mcp list workspace` exit before listing with `You are not logged in,
please log in with kiro-cli login`. The current public docs describe `mcp list`
as configured-server listing and document interactive `/mcp` as loaded-server/tool
status, but do not publish a stable machine-readable or human-output grammar for
`mcp list` or a non-interactive effective-load probe. No credentials or
interactive login were used. Effective observation is therefore unverified;
registration and stage completion remain blocked until official authenticated
runtime evidence closes this gap.
