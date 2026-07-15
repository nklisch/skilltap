---
id: epic-expanded-harness-support-file-managed-kiro
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/kiro-cli-2.12.2.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
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

## Implementation amendment

The source-direct attestation establishes Kiro CLI `2.12.2`, the exact
`kiro-cli --version` bytes, `${KIRO_HOME}` relocation, global/workspace Agent
Skills roots, global/workspace `mcp.json` declarations, precedence, supported
MCP fields, hot reload behavior, and the separate Power boundary. It also
establishes that `kiro-cli mcp list` requires login and that the public contract
does not provide a stable non-interactive effective-load grammar. The adapter
therefore consumes the attested declaration contract without invoking Kiro's
MCP command, login, trust, or interactive `/mcp` surface.

- Promoted `kiro.rs` and `kiro_managed.rs` from provisional test-only modules to
  production modules, exported `KiroAdapter`, `KiroManagedProjection`, and
  `KiroSkillProjection`, and registered Kiro with exact profile `kiro-2-12-2`
  and default binary `kiro-cli`.
- The exact profile exposes only the attested observe, complete-skill, and MCP
  declaration surfaces in both global and project scopes. It has no native
  lifecycle, Power, authentication, trust, or effective-status authority.
- Added an explicit `ManagedDeclarationContract` covering exactly
  `ManagedDocument` and `CompleteSkillTree`. Kiro managed plugin writes are
  partial declaration operations: foreground `--yes` is required, the daemon
  leaves them pending, and status reports declared ownership with effective
  state unverified.
- Removed the unused Kiro effective-probe implementation rather than leaving a
  login-bearing invocation seam. Empty documented roots are valid observations
  before the first declaration, so source registration remains control-plane
  only.
- Preserved the existing Kiro codec hard boundaries: complete skill trees,
  `mcpServers`-only writes, unrelated/unknown fields, disabled state, tool
  filters, references, conflicts, drift, required unsupported components,
  Power exclusion, confined rollback, and repeat idempotence.
- Added compiled isolated acceptance for acknowledgment blocking, exact global
  and project declarations, project skill links, effective-unverified status,
  daemon zero target writes, no MCP/login/cache/Power process or filesystem
  activity, immediate repeats, and adjacent/unknown version zero writes.

## Verification

- `cargo test -p skilltap-harnesses --all-targets` — passed.
- `cargo test -p skilltap --test compiled_binary kiro_` — 2 passed.
- The final workspace/all-target verification is recorded by the parent
  acceptance checkpoint.
