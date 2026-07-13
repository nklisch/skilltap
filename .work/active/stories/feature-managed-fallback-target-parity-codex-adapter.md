---
id: feature-managed-fallback-target-parity-codex-adapter
kind: story
stage: review
tags: []
parent: feature-managed-fallback-target-parity
depends_on: [feature-managed-fallback-target-parity-contract-evidence]
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

Implement Unit 2 of the managed-fallback-target-parity feature design against
the **amended** contract (`feature-managed-fallback-target-parity-contract-
evidence`): a `CodexManagedProjection` struct that implements the
single-method `ManagedProjectionPort::plan` by **relocating** — not rewriting
— the existing Codex managed-project helpers out of CLI and into
`skilltap-harnesses`. `CodexAdapter::managed_projection()` returns the static
ref. The relocation is behavior-preserving for install/update paths; removal
is corrected to drop the approved contract's mandatory source acquisition
(see Behavior change for removal below).

The CLI keeps its existing `plan_managed_codex_project_lifecycle` orchestrator
for now (the orchestrator story removes it). To avoid a behavioral fork during
this story, the CLI orchestrator and the new adapter share the same relocated
helpers: the helpers move to `crates/harnesses/src/adapters/codex_managed.rs`
(or a private `codex_managed_helpers` submodule) and the CLI function imports
them. This keeps install/update a pure relocation.

Parent design: `feature-managed-fallback-target-parity` Unit 2, as amended by
the Implementation discovery and contract amendment section. The contract
amendment this story depends on is
`feature-managed-fallback-target-parity-contract-evidence`.

### Behavior change for removal

The amended contract models removal as `ManagedProjectionInput::Remove`, which
carries no `ResolvedSourceCheckout`. This is an intentional, contract-driven
change to the removal path: plugin removal already planned from the prior
manifest without source (the existing Codex path proves this works);
marketplace removal now also plans from the prior manifest plus current
filesystem observation of `.agents/plugins/marketplace.json`, and no longer
fails with `managed_project_source_missing` when the marketplace source is
unreachable but its catalog projection is still observable. The
`managed_project_source_missing` error remains a typed variant for install/
update, where source acquisition is genuinely required. The Codex regression
suite must be updated to reflect that marketplace removal no longer requires
source (the previously source-gated removal test moves to a source-absent
fixture that succeeds against an observable catalog).

## Units

- `crates/harnesses/src/adapters/codex_managed.rs` (new):
  `CodexManagedProjection` struct implementing the amended single-method
  `ManagedProjectionPort::plan`, plus the relocated private helpers (catalog
  read at a checkout root, plugin-tree read, component-projection planning,
  MCP-config planning, plugin-root-relative gating) and the adapter-private
  path/format constants.
- `crates/harnesses/src/adapters/codex.rs` (modified): override
  `managed_projection()` to return `CodexManagedProjection::static_ref()`.
- `crates/cli/src/application.rs` (modified): the Codex-specific free
  functions (`resolve_codex_marketplace_source`, `read_codex_catalog_at_root`,
  `plan_codex_component_projections`, `plan_codex_mcp_config`,
  `mcp_depends_on_plugin_root`, `read_complete_codex_plugin`,
  `ResolvedCodexMarketplace`, `CodexComponentProjectionPlan`,
  `CodexMcpConfigPlan`) are deleted from CLI; `plan_managed_codex_project_
  lifecycle` imports the relocated helpers from harnesses for the duration of
  this story (the orchestrator story deletes it).

### Amended port consumption

`CodexManagedProjection::plan` matches on `context.input`:

- `ManagedProjectionInput::Apply { checkout }`:
  - `ResourceKind::Marketplace`: read the catalog at `checkout.root()` via
    `CODEX_CATALOG_DESTINATIONS` → one `ManagedFileWrite` whose `desired` is
    the catalog bytes (relocated from the marketplace branch of
    `plan_managed_codex_project_lifecycle` and `read_codex_catalog_at_root`).
  - `ResourceKind::Plugin`: derive the plugin selector from
    `context.request.name.as_str()` (it carries the spec's `native_name`,
    verified at `application.rs:1199-1203`). The orchestrator resolves the
    selected marketplace source into the one authoritative checkout; resolve
    the catalog at `checkout.root()`, call `catalog.plugin_source(selector.plugin(),
    checkout.root())` for the contained plugin root, and pass
    `checkout.source()` as provenance to `read_complete_codex_plugin`. Then
    plan skill trees + MCP config (relocated from
    `plan_codex_component_projections` + `plan_codex_mcp_config`).
  - For both, the returned `ManagedProjectionPlan` carries `manifest`,
    `current_fingerprint`, and `desired_fingerprint` directly from the
    relocated helpers (the intermediate `CodexComponentProjectionPlan` /
    `CodexMcpConfigPlan` types fold into the plan's evidence fields and are
    deleted).
- `ManagedProjectionInput::Remove`: plan from `context.prior` plus current
  filesystem observation only. `ResourceKind::Plugin` reuses the relocated
  removal branch verbatim (it already passes `plugin: None` and plans from
  `prior`). `ResourceKind::Marketplace` reads the current catalog bytes from
  the project filesystem, sets `desired_fingerprint: None`, and produces a
  `ManagedFileWrite` with `desired: None` — no source acquisition, no
  `managed_project_source_missing` failure.

Adapter-private constants (declared in `codex_managed.rs`):

```rust
pub(crate) const CODEX_CATALOG_DESTINATIONS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
pub(crate) const CODEX_MCP_DESTINATION: &str = ".codex/config.toml";
```

## Implementation notes

- The amended port (`feature-managed-fallback-target-parity-contract-evidence`)
  collapses acquire/project into one `plan` method and passes the
  caller-resolved `ResolvedSourceCheckout` via `ManagedProjectionInput::Apply`.
  The adapter no longer calls `resolve_git_skill_source` or any source
  resolver; it reads catalog/plugin trees from `checkout.root()`. This is the
  discovery's gap 1 fix.
- `ManagedCodexCatalog` and `ManagedCodexCatalogError` already live in
  harnesses (`crates/harnesses/src/managed_codex_project.rs`) and are
  unchanged.
- The plugin-root-relative MCP executable gate (`mcp_depends_on_plugin_root`)
  relocates with the MCP planning helper. Its evidence code
  (`plugin_root_relative_mcp_omitted`) stays the exact string and is emitted
  as a `ManagedProjection::Omitted` entry inside `plan.manifest` (replacing
  the old `plan.omitted` field that the amendment removed).
- `ComponentDeclaration` is already a core type
  (`crates/core/src/plugin_graph.rs:20`); the adapter reads declarations via
  `CodexPluginGraphReader` and uses them internally (they no longer cross the
  port boundary, since `AcquiredProjection` was removed by the amendment).
- The relocated helpers populate `plan.manifest`, `plan.current_fingerprint`,
  and `plan.desired_fingerprint` directly — the values the discovery's gap 3
  identified as missing. The CLI orchestrator continues to compute these via
  the relocated helpers until the orchestrator story flips dispatch onto the
  port; the port simply returns them.
- Install/update paths must remain byte-identical to the existing Codex
  orchestrator output (operations, entries, seeds, error codes, manifest,
  fingerprints). The Codex managed-project test suite in
  `crates/cli/src/application/tests.rs` is the regression bar for those paths.
- Removal behavior changes per the Scope section (marketplace removal no
  longer requires source). The relevant regression test is updated, not
  deleted; the new fixture proves removal succeeds against an observable
  catalog with the source absent.
- The CLI's `plan_managed_codex_project_lifecycle` keeps working by importing
  the relocated helpers; it is not yet deleted (the orchestrator story deletes
  it after flipping the dispatch).

## Acceptance criteria

- [x] `CodexManagedProjection` implements the amended single-method
      `ManagedProjectionPort::plan` and `CodexAdapter::managed_projection()`
      returns its static ref.
- [x] For `ManagedProjectionInput::Apply`, every existing Codex managed-
      project install/update test in `crates/cli/src/application/tests.rs`
      passes without modification to its assertions — the tests at lines 582
      (publication failure retry + noop), 725 (tree-limit revalidation),
      833-969 (pending-attempt recovery for install/update), 1360-1506
      (ownership validation), and the drift/unowned/unsupported-partial
      cases all stay green.
- [x] The Codex catalog destination search order (`.agents/plugins/
      marketplace.json` then `.claude-plugin/marketplace.json`), the MCP
      destination (`.codex/config.toml`), the MCP TOML table name
      (`mcp_servers`), and the plugin-root-relative evidence code
      (`plugin_root_relative_mcp_omitted`) are byte-identical, pinned by
      the existing tests. The evidence code is emitted as a
      `ManagedProjection::Omitted` entry inside `plan.manifest`.
- [x] For `ManagedProjectionInput::Remove`, plugin removal behaves
      byte-identically to today (plans from `prior`, no source). Marketplace
      removal no longer fails with `managed_project_source_missing` when the
      source is absent; the previously source-gated removal regression test
      is updated to a source-absent fixture that succeeds against an
      observable catalog projection, and that change is documented in the
      test.
- [x] The returned `ManagedProjectionPlan` carries `manifest`,
      `current_fingerprint`, and `desired_fingerprint` matching the values
      the existing Codex helpers compute, so the CLI orchestrator (still
      driving Codex through the relocated helpers) and the port produce
      identical evidence.
- [x] `git grep -n "plan_codex_component_projections\|plan_codex_mcp_config\
      |read_complete_codex_plugin\|read_codex_catalog_at_root\|mcp_depends_\
      on_plugin_root" crates/cli/` returns no matches (helpers fully
      relocated to harnesses).
- [x] `CodexComponentProjectionPlan` and `CodexMcpConfigPlan` no longer
      exist; their fields fold into `ManagedProjectionPlan`'s writes and
      evidence fields.
- [x] `cargo test --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --all -- --check`, and `git diff --check` pass.

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

## Implementation completion notes

- Execution capability: highest, inherited from the active autopilot run; this
  relocation controls persisted ownership evidence and target-native file
  codecs used by every managed Codex project mutation.
- Review weight: standard (project/autopilot default).
- Dispatch: direct implementation only, as required by the caller; no agent or
  peer delegation.
- Files changed: added
  `crates/harnesses/src/adapters/codex_managed.rs`; updated the Codex adapter,
  adapter exports, the temporary CLI Codex orchestrator integration,
  `crates/cli/src/application/tests.rs`, and this story.
- Tests added: `managed_marketplace_removal_uses_observed_projection_without_source`
  proves removal succeeds after the upstream checkout disappears while the
  owned catalog projection remains observable. Existing managed publication
  retry/no-op, tree-limit revalidation, pending install/update recovery,
  ownership, drift, unowned, partial-acknowledgment, and unsupported-component
  assertions were not weakened.
- Simplification: removed all Codex catalog/plugin/MCP path and codec helpers
  from CLI, removed the `ResolvedCodexMarketplace`,
  `CodexComponentProjectionPlan`, and `CodexMcpConfigPlan` intermediates, and
  folded writes, manifest, and aggregate fingerprints directly into
  `ManagedProjectionPlan`.
- Contract integration: the temporary Codex orchestrator resolves one
  `ResolvedSourceCheckout`, invokes `CodexManagedProjection::plan` directly,
  and translates only normalized writes into the existing revalidated
  execution-port entries. Registry-driven shared dispatch remains owned by the
  orchestrator story.
- Narrow behavior corrections: marketplace removal is source-free as designed.
  Plugin provenance now records the one authoritative selected marketplace
  checkout source instead of synthesizing a second plugin-subdirectory source;
  projected bytes, operations, manifests, and fingerprints are unchanged. The
  catalog-missing diagnostic keeps the approved typed code and uses the
  contract's target-neutral summary rather than the old Codex-specific wording.
- Verification: 563 workspace tests passed; the six focused managed lifecycle
  tests passed ten consecutive runs (60 executions); workspace clippy with
  warnings denied, formatting, helper/type elimination greps, and diff checks
  passed.
- Discrepancies from design: none beyond the two explicitly approved/narrow
  contract corrections above.
- Adjacent issues parked: none.

## Out of scope

- The target-agnostic orchestrator and CLI dispatch flip (Unit 3 /
  `feature-managed-fallback-target-parity-orchestrator`). The CLI
  `plan_managed_codex_project_lifecycle` is preserved through this story.
- The shared acceptance matrix and fake-adapter proof (Unit 4 /
  `feature-managed-fallback-target-parity-acceptance`).
- Claude managed-project lifecycle changes (Claude's state is preserved
  as-is; Claude is not migrated in this feature scope).
