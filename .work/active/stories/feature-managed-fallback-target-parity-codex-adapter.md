---
id: feature-managed-fallback-target-parity-codex-adapter
kind: story
stage: drafting
tags: []
parent: feature-managed-fallback-target-parity
depends_on: [feature-managed-fallback-target-parity-contract]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Codex Managed-Projection Adapter

## Scope

Implement Unit 2 of the managed-fallback-target-parity feature design: a
`CodexManagedProjection` struct that implements `ManagedProjectionPort` by
**relocating** — not rewriting — the existing Codex managed-project helpers
out of CLI and into `skilltap-harnesses`. `CodexAdapter::managed_projection()`
returns the static ref. Behavior is byte-identical; only the module home
changes.

The CLI keeps its existing `plan_managed_codex_project_lifecycle` orchestrator
for now (Unit 3 removes it). To avoid a behavioral fork during this story,
the CLI orchestrator and the new adapter port share the same relocated
helpers: the helpers move to `crates/harnesses/src/adapters/codex_managed.rs`
(or a private `codex_managed_helpers` submodule) and the CLI function imports
them. This makes the story a pure relocation.

Parent design: `feature-managed-fallback-target-parity` Unit 2.

## Units

- `crates/harnesses/src/adapters/codex_managed.rs` (new):
  `CodexManagedProjection` struct implementing `ManagedProjectionPort`, plus
  the relocated private helpers (catalog read, plugin-tree read,
  component-projection planning, MCP-config planning, plugin-root-relative
  gating) and the adapter-private path/format constants.
- `crates/harnesses/src/adapters/codex.rs` (modified): override
  `managed_projection()` to return `CodexManagedProjection::static_ref()`.
- `crates/cli/src/application.rs` (modified): the Codex-specific free
  functions (`resolve_codex_marketplace_source`, `read_codex_catalog_at_root`,
  `plan_codex_component_projections`, `plan_codex_mcp_config`,
  `mcp_depends_on_plugin_root`, `read_complete_codex_plugin`,
  `ResolvedCodexMarketplace`, `CodexComponentProjectionPlan`,
  `CodexMcpConfigPlan`) are deleted from CLI; `plan_managed_codex_project_
  lifecycle` imports the relocated helpers from harnesses for the duration of
  this story (Unit 3 deletes it).

The full code shape is in the parent feature's Unit 2 design body. The key
relocations (verified line numbers in the grounding summary):

- Marketplace acquisition: `application.rs:1467-1577` marketplace branch +
  `resolve_codex_marketplace_source` (1777) + `read_codex_catalog_at_root`
  (1773) → `CodexManagedProjection::acquire` returning
  `AcquiredProjection::MarketplaceCatalog { bytes, fingerprint, source,
  installed_revision }`.
- Plugin acquisition: `application.rs:1577+` plugin branch +
  `read_complete_codex_plugin` (2301) → `CodexManagedProjection::acquire`
  returning `AcquiredProjection::Plugin { tree, fingerprint, declarations,
  source, installed_revision }`.
- Plugin projection: `plan_codex_component_projections` (1818) +
  `plan_codex_mcp_config` (2050) + `mcp_depends_on_plugin_root` (2253) →
  `CodexManagedProjection::project` returning `ManagedProjectionPlan {
  trees, files, omitted }`. The intermediate `CodexComponentProjectionPlan`
  (1975) and `CodexMcpConfigPlan` (2246) types collapse into
  `ManagedProjectionPlan` and are deleted.

Adapter-private constants (declared in `codex_managed.rs`):

```rust
pub(crate) const CODEX_CATALOG_DESTINATIONS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
pub(crate) const CODEX_MCP_DESTINATION: &str = ".codex/config.toml";
```

## Implementation notes

- `ManagedCodexCatalog` and `ManagedCodexCatalogError` already live in
  harnesses (`crates/harnesses/src/managed_codex_project.rs`) and are
  unchanged.
- Local-vs-git source resolution moves behind the shared
  `SourceRevisionResolver` the port receives. Codex no longer calls
  `resolve_git_skill_source` directly; the resolver is the same `GitSource
  RevisionResolver` machinery, supplied by the caller. This keeps the
  helper testable in isolation.
- The plugin-root-relative MCP executable gate (`mcp_depends_on_plugin_root`)
  relocates with `plan_codex_mcp_config`. Its evidence code
  (`plugin_root_relative_mcp_omitted`) stays the exact string.
- `ComponentDeclaration` is already a core type
  (`crates/core/src/plugin_graph.rs:20`), so the port's
  `AcquiredProjection::Plugin::declarations` field carries it directly.
- This story must not change any user-facing behavior or error code. The
  Codex managed-project test suite in `crates/cli/src/application/tests.rs`
  is the regression bar.
- The CLI's `plan_managed_codex_project_lifecycle` keeps working by importing
  the relocated helpers; it is not yet deleted (Unit 3 deletes it after
  flipping the dispatch).

## Acceptance criteria

- [ ] `CodexManagedProjection` implements `ManagedProjectionPort` and
      `CodexAdapter::managed_projection()` returns its static ref.
- [ ] Every existing Codex managed-project test in
      `crates/cli/src/application/tests.rs` passes without modification to
      its assertions — the tests at lines 582 (publication failure retry +
      noop), 725 (tree-limit revalidation), 833-969 (pending-attempt
      recovery for install/update), 1360-1506 (ownership validation), and
      the drift/unowned/unsupported-partial cases all stay green.
- [ ] The Codex catalog destination search order (`.agents/plugins/
      marketplace.json` then `.claude-plugin/marketplace.json`), the MCP
      destination (`.codex/config.toml`), the MCP TOML table name
      (`mcp_servers`), and the plugin-root-relative evidence code
      (`plugin_root_relative_mcp_omitted`) are byte-identical, pinned by the
      existing tests.
- [ ] `git grep -n "plan_codex_component_projections\|plan_codex_mcp_config\
      |read_complete_codex_plugin\|read_codex_catalog_at_root\|mcp_depends_\
      on_plugin_root" crates/cli/` returns no matches (helpers fully
      relocated to harnesses).
- [ ] `CodexComponentProjectionPlan` and `CodexMcpConfigPlan` no longer
      exist; their fields fold into `ManagedProjectionPlan`.
- [ ] `cargo test --workspace --all-targets` and `cargo clippy --workspace
      --all-targets -- -D warnings` pass.

## Implementation discovery

Implementation stopped before code changes because the completed Unit 1
contract cannot carry the evidence required to relocate the current Codex
behavior without changing it:

1. `ManagedAcquisitionContext::revision_resolver` is a
   `SourceRevisionResolver`, whose only operation returns a
   `ResolvedRevision`. It cannot return the checked-out, confined source root
   that `read_codex_catalog_at_root` and `read_complete_codex_plugin` must
   read. Recreating `resolve_git_skill_source` inside the adapter would bypass
   the supplied port, duplicate the Git process/cache boundary, and contradict
   the design decision that acquisition reuses shared Git resolution.
2. A fresh plugin install has no source on the plugin resource or
   `NativeLifecycleRequest`; the existing orchestrator resolves the selected
   marketplace resource from inventory and passes that marketplace's source to
   catalog lookup. `ManagedAcquisitionContext` carries neither the documents
   nor a typed resolved marketplace source, so `CodexManagedProjection::acquire`
   cannot locate `plugin@marketplace` without a CLI-owned Codex side channel.
3. `ManagedProjectionPlan` carries writes and omissions only. The existing
   Codex projection helper also returns the exact current aggregate
   fingerprint, desired aggregate fingerprint, and `ManagedProjection::Mcp`
   entries with per-server fingerprints. Those values drive ownership, drift,
   pending-attempt recovery, update-required checks, and persisted projection
   state. They cannot be reconstructed from `ManagedFileWrite` without parsing
   Codex TOML in CLI, and hashing the whole file would include unmanaged
   configuration and change existing semantics. `AcquiredProjection::Plugin`
   fingerprints the source plugin tree, not the projected managed surfaces, so
   it is not a compatible substitute.

The contract needs a typed revision before Unit 2 can proceed: acquisition must
receive or resolve a checked-out source root plus revision; fresh plugin
acquisition must receive the selected marketplace source explicitly; and the
projection result must carry the target-neutral current/desired fingerprints
and complete managed-projection manifest (or an equivalent typed evidence
object). Removal should also be modeled without requiring source acquisition
when the prior manifest is sufficient. These are normalized domain values, not
Codex codecs or paths.

Dispatch: direct-read only, as required by the caller. No production or test
files changed, and no assertions were weakened. The story returned to
`stage:drafting` so design can amend the contract rather than add stringly
escape hatches.

## Out of scope

- The target-agnostic orchestrator and CLI dispatch flip (Unit 3 /
  `feature-managed-fallback-target-parity-orchestrator`). The CLI
  `plan_managed_codex_project_lifecycle` is preserved through this story.
- The shared acceptance matrix and fake-adapter proof (Unit 4 /
  `feature-managed-fallback-target-parity-acceptance`).
- Claude managed-project lifecycle changes (Claude's state is preserved
  as-is; Claude is not migrated in this feature scope).
