---
id: feature-managed-fallback-target-parity
kind: feature
stage: review
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-cross-harness-materialization, epic-expanded-harness-support-registry]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-13
---

# Complete Managed Fallback Target Parity

## Brief

Complete the cross-harness promise for an explicitly selected plugin when the
target has no faithful native distribution or native plugin lifecycle. The
current production publication path covers Codex project skills and portable
MCP configuration through a Codex-specialized CLI orchestrator
(`plan_managed_codex_project_lifecycle`); extract a target-agnostic managed
projection lifecycle driven by an adapter port so every supported target can
use its documented skill and MCP load surfaces without requiring that target
to provide a marketplace or plugin manager and without duplicating the
acquisition, ownership, drift, update, or removal machinery per adapter.

Native dual distributions remain preferred and independently tracked. Managed
fallback owns acquisition, revision, projection, drift, update, and removal
only for the target that lacks a native distribution. Complete skill
directories remain indivisible resources, unsupported required components
remain blocked, optional loss remains visible and acknowledgment-gated, and no
adapter writes undocumented caches.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: shared managed-projection foundation feature. The four
  concrete adapter families (`epic-expanded-harness-support-file-managed`,
  `-native-coexistence`, `-configuration-constrained`, `-trust-interactive`)
  and the Pi compound adapter all `depends_on` this feature and consume the
  `ManagedProjectionPort` it introduces; they do not each reinvent
  acquisition, ownership, drift, update, or removal.
- Depends on: `epic-cross-harness-materialization` (delivers the pure
  `PublicationBatch` / `PublicationSink` / `LoadVerifier` / ownership-refresh
  primitives this feature reuses verbatim) and `epic-expanded-harness-support-
  registry` (delivers `TargetRegistry`, `HarnessAdapter`, and the optional
  port-trait pattern this feature extends).

## Simplification opportunity

- Eliminate the Codex-specialized CLI orchestrator
  `plan_managed_codex_project_lifecycle` and its hardcoded
  `HarnessId::new("codex")` literals by generalizing it to one
  target-agnostic `plan_managed_project_lifecycle` that dispatches through an
  adapter port. The orchestrator, ownership validation, drift detection,
  pending-attempt recovery, observation, manifest building, foreground
  acknowledgment, publication, and load verification collapse into one shared
  code path instead of being copied per adapter.

## Foundation references

- `docs/VISION.md` — Native First, Faithfulness Before Portability, Explicit
  Loss, Observable Ownership.
- `docs/SPEC.md` — Plugin Lifecycle, Marketplace Lifecycle, Ownership and
  Removal, Standalone Skills (`managed/` artifacts, managed projection
  lifecycle, drift, update, removal).
- `docs/ARCH.md` — Plugin Publication Boundary, Harness Adapter Contract,
  Capability Detection, Apply Flow, Error Model.
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Expanded Target Set,
  Cross-Harness Component Matrix, MCP Mapping, Codex Contract (the
  projection/MCP destination and catalog format this feature extracts from).

## Grounding summary

Probed the Codex-specialized managed-project pipeline this feature generalizes
(verified, not assumed):

- The CLI orchestrator `plan_managed_codex_project_lifecycle` lives at
  `crates/cli/src/application.rs:1357` and hardcodes
  `let codex = HarnessId::new("codex").expect(...)` at line 1363. Its single
  call site is `crates/cli/src/application/lifecycle.rs:520`, gated one block
  above (line 491) by `adapter.managed_project_lifecycle()` plus a
  `Scope::Project(project)` match — the gate is already target-agnostic; only
  the planner body is Codex-specific.
- Codex-specific helpers collocated with it in `application.rs`:
  `resolve_codex_marketplace_source` (1777), `read_codex_catalog_at_root`
  (1773, tries `.agents/plugins/marketplace.json` then
  `.claude-plugin/marketplace.json`), `plan_codex_component_projections`
  (1818, builds skill `ManagedProjectPluginWrite`s from declarations),
  `plan_codex_mcp_config` (2050, merges into `.codex/config.toml`
  `mcp_servers` TOML with `mcp_depends_on_plugin_root` gating at 2253),
  `read_complete_codex_plugin` (2301), `ManagedCodexProjectPlanContext`
  (1347), `ResolvedCodexMarketplace` (1720), `CodexComponentProjectionPlan`
  (1975), `CodexMcpConfigPlan` (2246).
- Target-agnostic helpers already collocated (no Codex literal):
  `observe_managed_project_tree` (1996),
  `managed_project_tree_observation_limits` (1983),
  `managed_projection_manifest` (2019), `append_projection_tree` (2037),
  `managed_project_error` (2417), `previously_applied` (2407).
- `validate_managed_project_ownership` (2353) is pure but hardcodes
  `HarnessId::new("codex")` twice (ownership binding lookup); its drift,
  unowned, update-required, and pending-attempt-recovery logic is otherwise
  target-agnostic.
- The execution/revalidation boundary is already a target-agnostic port:
  `ManagedProjectLifecyclePort`, `ManagedProjectLifecycleEntry`,
  `ManagedProjectFileWrite`, `ManagedProjectPluginWrite`, and the
  `ManagedProjectFileSystem` trait alias live in
  `crates/cli/src/application/execution.rs:207-241` and implement
  `ExecutionPort::revalidate` / `apply` (the revalidated-execution-port
  pattern). No Codex literal lives there.
- The state model is target-agnostic: `ManagedProjection::{Skill, Mcp,
  Omitted}` in `crates/core/src/storage/state.rs:16`; `PendingManagedAttempt`
  recovery in `crates/core/src/storage/state.rs:308-439`; both keyed by
  `HarnessId` already.
- The publication boundary is target-agnostic:
  `PublicationBatch`/`PublicationSink`/`LoadVerifier`/`record_verified_publication`
  in `crates/core/src/publication.rs`; `ManagedArtifactRepository` in
  `crates/core/src/storage/managed_artifact.rs:259`. The CLI wires these under
  the configuration lock; this feature does not touch them.
- The Codex catalog codec already lives in harnesses, not CLI:
  `ManagedCodexCatalog` / `ManagedCodexCatalogError` in
  `crates/harnesses/src/managed_codex_project.rs`, re-exported from
  `crates/harnesses/src/lib.rs:28`. The CLI imports it at
  `application.rs:60` only to call `.parse` / `.plugin_source` /
  `.into_bytes`. This is the natural home for the rest of the Codex projection
  codec.
- Git-source resolution is already adapter-shared infrastructure:
  `GitSourceRevisionResolver` in `crates/harnesses/src/update_resolution.rs`
  (re-exported at `lib.rs:34`); `resolve_git_skill_source` in
  `application.rs:756` is its CLI-side caller and works for any source.
- `HarnessAdapter` (`crates/harnesses/src/registry.rs`) already carries the
  managed-fallback gate `managed_project_lifecycle() -> bool` (default false,
  true on `CodexAdapter` at `adapters/codex.rs:94`) plus the established
  optional-port pattern (`native_lifecycle`, `instruction_bridge`,
  `skill_projection` all return `Option<&dyn Port>`). This feature adds one
  more optional port in the same shape.

Foundation docs already describe the intended future state (SPEC `managed/`
and Standalone Skills describe managed projection; ARCH Plugin Publication
Boundary and Harness Adapter Contract describe the port-trait shape;
HARNESS-CONTRACTS Common Capability Model describes required-vs-optional
component semantics). This feature is code-first against those
already-rolled-forward assertions; no foundation-doc edits are required at
design time.

## Design decisions

- **One adapter port, not a CLI dispatch table.** A new
  `ManagedProjectionPort` trait in `skilltap-harnesses` exposes the
  target-specific surface (acquisition + projection) behind the same
  `Option<&dyn Port>` shape as the existing optional ports. The CLI
  orchestrator dispatches through `adapter.managed_projection()` and never
  matches on the harness id string. Chosen over a CLI-side `match target`
  because every new adapter would otherwise reopen that match and duplicate
  the ownership/drift/idempotency scaffolding around its target-specific
  code; the adapter-family pattern was just rejected for the same reason in
  the registry feature.
- **Two responsibilities on the port, no more.** The port owns exactly the
  two things that are genuinely target-specific: (1) **acquire** the source
  content (catalog format, plugin→tree resolution, per-target source roots)
  into a normalized `AcquiredProjection`, and (2) **project** that content to
  target-bound writes (skill destination paths via the existing
  `SkillProjectionPort`, MCP document codec, omitted-component evidence).
  Everything else (state lookup, operation id, ownership validation, drift,
  pending-attempt recovery, observation, manifest building, foreground
  acknowledgment, publication, load verification) stays in the shared
  orchestrator and core. This keeps the port minimal, avoids leaking Codex
  shape into the contract, and means a future file-managed adapter
  implements only acquire (load skill tree from git/local) and project
  (write to its skill root + merge into its MCP format).
- **MCP document codec behind a trait object, not a CLI branch.** Codex's
  `.codex/config.toml` + `mcp_servers` TOML format, JSONC variants, and
  reload-constrained formats all differ. The port returns target-bound
  `ManagedFileWrite`s whose `desired` bytes the adapter produces through its
  own codec; the orchestrator never parses or merges MCP documents. This
  preserves "no native codec in core, no target path logic in CLI".
- **Marketplace-catalog projection is a Codex adapter concern, not a shared
  concept.** `ResourceKind::Marketplace` (writing the catalog bytes to
  `.agents/plugins/marketplace.json`) is Codex's plugin-distribution
  representation. Future adapters (Gemini, OpenCode, ...) have no marketplace
  catalog concept; they acquire skills/MCP directly. The port's `acquire`
  therefore returns an `AcquiredProjection` enum (`Plugin { .. }` or
  `MarketplaceCatalog { bytes, .. }`) so a Codex adapter can project a
  catalog write while a file-managed adapter only ever returns `Plugin`.
  Adapters that cannot handle a requested kind return a typed
  `ManagedProjectionError::UnsupportedResourceKind`.
- **Ownership validation generalizes, not relocates.** `validate_managed_
  project_ownership` becomes parameterized by `&HarnessId` (the dispatch's
  resolved target id) instead of hardcoding `"codex"`. It stays in CLI as a
  pure helper called by the orchestrator — its dependencies
  (`Ownership::Skilltap`, `Provenance::Materialized`, `PendingManagedAttempt`)
  are already core types and the function does no I/O. Moving it to core is
  optional and deferred: it is not a native codec and its relocation earns no
  further decoupling until a second caller appears.
- **Acquisition reuses shared git resolution.** The port's `acquire` receives
  a `&dyn SourceRevisionResolver` (already the abstraction behind
  `GitSourceRevisionResolver`) and the local-source path; adapters do not
  re-implement git clone/checkout. This keeps acquisition deterministic and
  bounded across targets.
- **Foreground acknowledgment stays orchestrator-owned.** The
  `partial_operation_requires_acknowledgment` gate and the
  required-unsupported block fire from the shared orchestrator based on the
  `ManagedProjectionPlan` the adapter returns (which lists `Omitted` entries
  and surfaces required-unsupported as `Err`). The adapter classifies; the
  orchestrator enforces. This keeps acknowledgment semantics uniform across
  targets.
- **No new state shape.** `ManagedProjection`, `PendingManagedAttempt`,
  `TargetResourceState`, and `ResourceState` are unchanged. The wire format
  and `STATE_SCHEMA_VERSION` stay; only the planning path that produces
  projections changes. This protects the in-flight `3.0.0` release and every
  existing drift/unowned/pending test.
- **First-party plugin bootstrap stays narrow.** The marketplace-catalog
  projection is part of Codex's adapter implementation, not a general
  capability advertisement. `DistributionSurface::FirstPartyPlugin` continues
  to gate `bootstrap` exactly as today; managed fallback projection never
  implies bootstrap eligibility.

## Architectural choice

**Chosen**: extend the registry's optional-port pattern with
`ManagedProjectionPort`. The port trait and its adapter accessor live in
`skilltap-harnesses` (`crates/harnesses/src/managed_projection.rs`); the pure
supporting types (`AcquiredProjection`, `ManagedProjectionPlan`,
`ManagedPluginWrite`, `ManagedFileWrite`, `ManagedProjectionError`,
`OmittedComponent`) live in `skilltap-core`
(`crates/core/src/managed_projection.rs`) because they reference only core
domain types and are produced/consumed across the core→harnesses→cli
boundary. The CLI orchestrator `plan_managed_project_lifecycle` is
target-agnostic and dispatches through `&dyn ManagedProjectionPort`. Adding a
managed-fallback target is one adapter module implementing the port plus one
`HarnessAdapter::managed_projection()` override — no CLI edit, no new branch.

**Rejected — CLI-side per-target match**: reopens a closed dispatch on every
adapter and forces each to reimplement ownership/drift/idempotency around its
codec. The registry feature just eliminated this shape for
`HarnessKind`/`HarnessPolicies`; reintroducing it here would contradict the
epic's "distinct adapters, shared lifecycle" decomposition.

**Rejected — universal managed-plugin format**: flattening every target's MCP
document and skill layout into one intermediate representation would violate
faithfulness (Codex TOML ≠ JSONC ≠ YAML; skill roots differ). The port keeps
each adapter's codec private and exchanges only normalized plans.

**Rejected — relocate ownership validation to core now**: the function is
pure and target-parameterizable, but it has one caller today. Relocation is
the natural revision point if a second caller (e.g., a future reconcile path)
appears; doing it now is speculative motion.

## Implementation Units

### Unit 1: Managed projection port contract and pure types

**Files**: `crates/core/src/managed_projection.rs` (new);
`crates/harnesses/src/managed_projection.rs` (new); re-exports in
`crates/core/src/lib.rs`, `crates/harnesses/src/lib.rs`, and the
`HarnessAdapter` trait in `crates/harnesses/src/registry.rs`.

**Story**: `feature-managed-fallback-target-parity-contract`.

Pure supporting types (core), referencing only existing core domain types
(`ArtifactTree`, `Fingerprint`, `Source`, `ResolvedRevision`,
`RelativeArtifactPath`, `ComponentId`, `EvidenceCode`, `NativeId`,
`AbsolutePath`, `DirectoryIdentity`):

```rust
// crates/core/src/managed_projection.rs
use crate::{
    domain::{
        AbsolutePath, ComponentDeclaration, ComponentId, EvidenceCode, Fingerprint,
        NativeId, RelativeArtifactPath, ResolvedRevision, Source,
    },
    runtime::DirectoryIdentity,
    storage::ArtifactTree,
};

/// A component this plan omits because the target cannot represent it
/// faithfully. Required-unsupported components never reach a plan; they
/// surface as `ManagedProjectionError::RequiredUnsupported` from `project`.
/// Optional omissions appear only when the orchestrator passed
/// `acknowledged: true`; otherwise the orchestrator blocks with
/// `partial_operation_requires_acknowledgment` before producing a plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedComponent {
    pub id: ComponentId,
    pub consequence: EvidenceCode,
}

/// Source content an adapter acquired for one resource kind. Codex returns
/// `Plugin` for plugin resources and `MarketplaceCatalog` for marketplace
/// resources; file-managed adapters return only `Plugin`.
#[derive(Clone, Debug)]
pub enum AcquiredProjection {
    /// A complete plugin tree plus its component declarations. The tree must
    /// include the top-level `SKILL.md` for every skill component; the
    /// orchestrator never reduces a skill to a single file.
    Plugin {
        tree: ArtifactTree,
        fingerprint: Fingerprint,
        declarations: Vec<ComponentDeclaration>,
        source: Source,
        installed_revision: Option<ResolvedRevision>,
    },
    /// Verbatim catalog/document bytes a target projects as-is (Codex
    /// marketplace catalog at `.agents/plugins/marketplace.json`). Adapters
    /// without a catalog concept never return this variant.
    MarketplaceCatalog {
        bytes: Vec<u8>,
        fingerprint: Fingerprint,
        source: Source,
        installed_revision: Option<ResolvedRevision>,
    },
}

impl AcquiredProjection {
    pub fn fingerprint(&self) -> &Fingerprint { /* match */ }
    pub fn source(&self) -> &Source { /* match */ }
    pub fn installed_revision(&self) -> Option<&ResolvedRevision> { /* match */ }
}

/// One complete skill-tree write the orchestrator revalidates and publishes.
/// Mirrors the existing execution-port shape but lives in core so the adapter
/// port can return it without depending on CLI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedPluginWrite {
    pub root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub desired_tree: Option<ArtifactTree>,
    pub expected_tree: Option<ArtifactTree>,
    pub expected_identity: Option<DirectoryIdentity>,
}

/// One MCP-config (or other managed) file write. `desired` bytes are
/// adapter-produced through the target's own codec; the orchestrator and
/// core never parse them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedFileWrite {
    pub root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub expected: Option<Vec<u8>>,
    pub desired: Option<Vec<u8>>,
}

/// The target-bound projection plan an adapter returns. Pure data; the
/// orchestrator wraps it into the execution-port `ManagedProjectLifecycle
/// Entry` and the state manifest.
#[derive(Clone, Debug, Default)]
pub struct ManagedProjectionPlan {
    pub trees: Vec<ManagedPluginWrite>,
    pub files: Vec<ManagedFileWrite>,
    pub omitted: Vec<OmittedComponent>,
}

/// Adapter-side projection errors. Codes are stable and surfaced verbatim by
/// the orchestrator's `managed_project_error` helper, preserving the existing
/// user-facing strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedProjectionError {
    UnsupportedResourceKind,
    SourceMissing,
    SourceUnavailable,
    CatalogMissing,
    CatalogInvalid,
    PluginMissing,
    PluginSourceInvalid,
    PluginUnreadable,
    McpInvalid,
    McpConflict,
    Drifted,
    Other { code: &'static str, summary: &'static str },
}

impl ManagedProjectionError {
    /// Stable error code string the orchestrator surfaces in `ErrorDetail`.
    pub fn code(&self) -> &'static str { /* matches the existing codes */ }
    pub fn summary(&self) -> &'static str { /* matches the existing summaries */ }
}

impl std::fmt::Display for ManagedProjectionError { /* ... */ }
impl std::error::Error for ManagedProjectionError {}
```

The adapter port (harnesses), in the same `Option<&dyn Port>` shape as the
existing optional ports:

```rust
// crates/harnesses/src/managed_projection.rs
use skilltap_core::{
    domain::{AbsolutePath, HarnessId, NativeId, ResourceKey, ResourceKind, Scope, Source},
    managed_projection::{AcquiredProjection, ManagedProjectionError, ManagedProjectionPlan},
    runtime::{JsonLimits, PlatformPaths},
};

use crate::lifecycle::NativeLifecycleRequest;

/// Context the orchestrator hands the adapter for source acquisition. The
/// filesystem bound is the existing core `ConfinedFileSystem` trait; the
/// resolver is `GitSourceRevisionResolver` (or its test double).
pub struct ManagedAcquisitionContext<'a> {
    pub target: &'a HarnessId,
    pub project: &'a AbsolutePath,
    pub paths: &'a PlatformPaths,
    pub resource_key: &'a ResourceKey,
    pub resource_kind: ResourceKind,
    pub request: &'a NativeLifecycleRequest,
    pub source: Option<&'a Source>,
    pub json_limits: JsonLimits,
    pub filesystem: &'a dyn skilltap_core::runtime::ConfinedFileSystem,
    pub revision_resolver: &'a dyn skilltap_core::runtime::SourceRevisionResolver,
}

/// Context the orchestrator hands the adapter for projection. `prior` is the
/// target's recorded managed projections (from `PendingManagedAttempt` when
/// a pending attempt matches, else the last-apply manifest); `acknowledged`
/// is the foreground `--yes` state — the adapter lists optional omissions
/// only when it is true.
pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub project: &'a AbsolutePath,
    pub acquired: &'a AcquiredProjection,
    pub prior: &'a [skilltap_core::storage::ManagedProjection],
    pub kind: crate::ManagedLifecycleKind, // see Unit 3: extracted CLI enum, re-homed
    pub acknowledged: bool,
    pub filesystem: &'a dyn skilltap_core::runtime::ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

/// Target-specific acquisition and projection for the managed fallback
/// lifecycle. Adapters implement this when
/// `HarnessAdapter::managed_project_lifecycle()` returns true.
pub trait ManagedProjectionPort: Sync {
    /// Resolve and load the source content for one resource. Adapters own
    /// catalog format, plugin→tree resolution, and per-target source roots;
    /// they delegate git checkout to `revision_resolver`.
    fn acquire(
        &self,
        context: &ManagedAcquisitionContext<'_>,
    ) -> Result<AcquiredProjection, ManagedProjectionError>;

    /// Project acquired content to target-bound writes plus the omitted-
    /// component manifest. Required-unsupported components return `Err`; the
    /// orchestrator blocks them even with `--yes`. Optional omissions appear
    /// in `plan.omitted` only when `context.acknowledged` is true (the
    /// orchestrator rejects the unacknowledged case before producing the
    /// plan).
    fn project(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}
```

The `HarnessAdapter` trait gains one defaulted accessor:

```rust
// crates/harnesses/src/registry.rs — add to `trait HarnessAdapter`
use crate::managed_projection::ManagedProjectionPort;

pub trait HarnessAdapter: Sync {
    // ...existing methods unchanged...

    /// Target-specific acquisition and projection for managed fallback.
    /// `None` (default) means this target never participates in managed
    /// project lifecycle, matching the default `managed_project_lifecycle() ->
    /// false`. Adapters that opt in return their port implementation.
    fn managed_projection(&self) -> Option<&'static dyn ManagedProjectionPort> {
        None
    }
}
```

**Implementation Notes**:

- This unit is purely additive: new modules, new types, one defaulted trait
  method. No existing public symbol is removed or renamed; no behavior
  changes. `CodexAdapter` does not yet override `managed_projection()`, so
  `plan_managed_codex_project_lifecycle` continues to drive Codex unchanged
  until Unit 3 flips the dispatch.
- `ManagedPluginWrite` / `ManagedFileWrite` intentionally mirror the
  CLI-private `ManagedProjectPluginWrite` / `ManagedProjectFileWrite`
  (`crates/cli/src/application/execution.rs:227-242`) so Unit 3 is a
  mechanical translation between the two at the orchestrator boundary. The
  CLI types stay private; the core types become the port's currency.
- `NativeLifecycleKind` is currently CLI-private (`application.rs:182`). Unit
  3 lifts the kind into a small shared enum (or reuses the existing
  `NativeLifecycleAction` from `crates/harnesses/src/lifecycle.rs:25`) so the
  port's `ManagedProjectionContext::kind` does not depend on CLI. The
  contract unit declares the type; Unit 3 performs the lift. To keep Unit 1
  independently compilable, `ManagedProjectionContext::kind` is spelled
  against a placeholder `crate::ManagedLifecycleKind` alias that Unit 3
  resolves (see Unit 3 notes).
- `ManagedProjectionError` carries the exact error codes the existing Codex
  path emits (`managed_project_source_missing`, `managed_project_catalog_
  missing`, `managed_project_catalog_invalid`, `managed_project_plugin_source_
  invalid`, `managed_project_mcp_invalid`, `managed_project_drifted`, ...).
  Unit 3 maps them one-to-one to the existing `ErrorDetail` codes so user-
  facing output is byte-identical.

**Acceptance Criteria**:

- [ ] `crates/core/src/managed_projection.rs` defines `AcquiredProjection`,
      `ManagedProjectionPlan`, `ManagedPluginWrite`, `ManagedFileWrite`,
      `OmittedComponent`, and `ManagedProjectionError` with the signatures
      above, referencing only existing public core types.
- [ ] `crates/harnesses/src/managed_projection.rs` defines
      `ManagedProjectionPort`, `ManagedAcquisitionContext`, and
      `ManagedProjectionContext` with the signatures above.
- [ ] `HarnessAdapter::managed_projection() -> Option<&'static dyn
      ManagedProjectionPort>` exists and defaults to `None`.
- [ ] An interface test (a throwaway test adapter like the registry contract
      story used) constructs a `ManagedProjectionPort` impl, calls
      `acquire`/`project`, and asserts the round-tripped plan equals the
      inputs — proving the port is object-safe and the types round-trip.
- [ ] `cargo test -p skilltap-core --lib` and `cargo test -p
      skilltap-harnesses --lib` pass; no existing test changes.

---

### Unit 2: Codex managed-projection adapter

**Files**: `crates/harnesses/src/adapters/codex_managed.rs` (new);
`crates/harnesses/src/adapters/codex.rs` (modified to override
`managed_projection()`); relocation of Codex projection helpers out of CLI.

**Story**: `feature-managed-fallback-target-parity-codex-adapter`.

A `CodexManagedProjection` struct implements `ManagedProjectionPort` by
relocating — not rewriting — the existing Codex functions. Behavior is
byte-identical; only the module home changes.

```rust
// crates/harnesses/src/adapters/codex_managed.rs
use skilltap_core::{
    domain::{AbsolutePath, NativeId, RelativeArtifactPath},
    managed_projection::{
        AcquiredProjection, ManagedFileWrite, ManagedPluginWrite,
        ManagedProjectionError, ManagedProjectionPlan, OmittedComponent,
    },
    runtime::{ConfinedFileSystem, JsonLimits},
    storage::ArtifactTree,
};

use crate::{
    managed_codex_project::{ManagedCodexCatalog, ManagedCodexCatalogError},
    managed_projection::{ManagedAcquisitionContext, ManagedProjectionContext},
    registry::ManagedProjectionPort as _, // trait import
};

pub struct CodexManagedProjection;

impl CodexManagedProjection {
    pub fn static_ref() -> &'static dyn crate::managed_projection::ManagedProjectionPort {
        &Self
    }
}

impl crate::managed_projection::ManagedProjectionPort for CodexManagedProjection {
    fn acquire(
        &self,
        ctx: &ManagedAcquisitionContext<'_>,
    ) -> Result<AcquiredProjection, ManagedProjectionError> {
        // Marketplace kind: resolve source (local|git) -> read catalog at
        //   .agents/plugins/marketplace.json | .claude-plugin/marketplace.json
        //   -> AcquiredProjection::MarketplaceCatalog { bytes, .. }.
        // Plugin kind: resolve marketplace source -> catalog -> plugin_source
        //   -> read_complete_codex_plugin -> AcquiredProjection::Plugin { tree,
        //   declarations, .. }.
        // Both branches relocate verbatim from resolve_codex_marketplace_source
        //   (application.rs:1777), read_codex_catalog_at_root (1773),
        //   read_complete_codex_plugin (2301), and the marketplace/plugin
        //   branches of plan_managed_codex_project_lifecycle (1467-1577).
        # todo!("Unit 2: relocate, do not rewrite")
    }

    fn project(
        &self,
        ctx: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
        // Marketplace kind: one ManagedFileWrite for the catalog destination.
        // Plugin kind: plan_codex_component_projections (skill trees) +
        //   plan_codex_mcp_config (.codex/config.toml mcp_servers merge +
        //   plugin-root-relative gating) -> ManagedProjectionPlan.
        // Relocated verbatim from application.rs:1818, 2050, 2253; the
        //   CodexComponentProjectionPlan / CodexMcpConfigPlan intermediate
        //   types collapse into ManagedProjectionPlan.
        # todo!("Unit 2: relocate, do not rewrite")
    }
}

/// Catalog destinations searched, in preference order. Stays adapter-private.
pub(crate) const CODEX_CATALOG_DESTINATIONS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];

/// Codex MCP config destination. Stays adapter-private.
pub(crate) const CODEX_MCP_DESTINATION: &str = ".codex/config.toml";
```

`CodexAdapter::managed_projection()` returns the static ref:

```rust
// crates/harnesses/src/adapters/codex.rs — add to impl HarnessAdapter for CodexAdapter
fn managed_projection(&self) -> Option<&'static dyn crate::managed_projection::ManagedProjectionPort> {
    Some(CodexManagedProjection::static_ref())
}
```

The CLI keeps its existing `plan_managed_codex_project_lifecycle` **for now**
(Unit 3 removes it). To avoid a behavioral fork during Unit 2, the CLI
function and the adapter port share the same relocated helpers: the helpers
move to `crates/harnesses/src/adapters/codex_managed.rs` (or a private
`codex_managed_helpers` submodule) and the CLI function imports them. This
makes Unit 2 a pure relocation: the CLI orchestrator still drives Codex, but
through shared harnesses-owned functions.

**Implementation Notes**:

- `ManagedCodexCatalog` and `ManagedCodexCatalogError` already live in
  harnesses and are unchanged.
- `mcp_depends_on_plugin_root` (the plugin-root-relative executable gate that
  produces `ManagedProjection::Omitted { consequence:
  plugin_root_relative_mcp_omitted }` when acknowledged) relocates with
  `plan_codex_mcp_config`. Its evidence code stays the exact string.
- The `read_complete_codex_plugin` helper (reads the plugin tree +
  declarations) relocates alongside; `ComponentDeclaration` is already a core
  type (`crates/core/src/plugin_graph.rs:20`).
- Local-vs-git source resolution moves behind the shared
  `SourceRevisionResolver` the port receives; Codex no longer calls
  `resolve_git_skill_source` directly but the resolver is the same machinery.
- The two intermediate types `CodexComponentProjectionPlan` and
  `CodexMcpConfigPlan` are deleted: their fields become
  `ManagedProjectionPlan { trees, files, omitted }` directly.

**Acceptance Criteria**:

- [ ] `CodexManagedProjection` implements `ManagedProjectionPort` and
      `CodexAdapter::managed_projection()` returns it.
- [ ] Every existing Codex managed-project test in
      `crates/cli/src/application/tests.rs` (drift, unowned, unsupported
      partial, pending-attempt recovery, MCP acknowledgment, tree-limit
      revalidation, publication failure retry, idempotent retry) passes
      without modification to its assertions — proving the relocation is
      behavior-preserving.
- [ ] The Codex catalog destination order, MCP destination, MCP TOML table
      name (`mcp_servers`), and plugin-root-relative evidence code are
      byte-identical (pinned by the existing tests).
- [ ] `git grep -n "plan_codex_component_projections\|plan_codex_mcp_config\
      |read_complete_codex_plugin" crates/cli/` returns no matches (helpers
      fully relocated to harnesses).

---

### Unit 3: Target-agnostic orchestrator and dispatch

**Files**: `crates/cli/src/application.rs` (modified — replace
`plan_managed_codex_project_lifecycle` with `plan_managed_project_lifecycle`,
generalize `validate_managed_project_ownership`, and consume the amended
single-method managed-projection port); `crates/cli/src/application/lifecycle.rs:520`
(modified — call the new orchestrator through `adapter.managed_projection()`).

**Story**: `feature-managed-fallback-target-parity-orchestrator`.

**Design correction**: this section supersedes the earlier Unit 3
`acquire`/`project` instructions. The active amended contract is
`ManagedProjectionPort::plan` with `ManagedProjectionInput::{Apply { checkout },
Remove}` and `ManagedProjectionPlan { trees, files, manifest,
current_fingerprint, desired_fingerprint }`. The CLI orchestrator resolves
source checkouts; adapters plan target-native writes and evidence in one pass;
the orchestrator consumes that evidence directly.

Target-neutral flow:

1. Resolve `registry.adapter(target)` and then `adapter.managed_projection()`;
   return typed attention errors for missing registry entries or unsupported
   managed projection targets. No CLI `match target` table is introduced.
2. Build the project scope, operation id, existing resource state,
   target-local state, pending-attempt-aware prior manifest, and `removal`
   flag for the resolved `HarnessId`.
3. Choose the port input from lifecycle kind:
   - marketplace add/update resolves the marketplace resource's source
     (`resource.source()` first, then existing target state) into
     `ResolvedSourceCheckout`;
   - plugin install/update resolves the selected marketplace source from
     inventory first and state second, then resolves that source into one
     authoritative checkout; no second plugin-subdirectory source is invented;
   - marketplace/plugin remove use `ManagedProjectionInput::Remove` and do no
     source resolution. Plugin removal keeps the existing unowned and missing-
     manifest guards; marketplace removal remains source-free per the contract
     amendment.
4. Resolve apply sources with the existing CLI source machinery:
   local sources validate into checkout roots; Git sources reuse
   `resolve_git_skill_source`; remote catalog payloads remain unsupported for
   managed plugin checkout. The checkout is owned by the orchestrator local
   variable, borrowed only for `port.plan`, then cloned for seed provenance and
   installed revision.
5. Build one `ManagedProjectionContext` (`target`, `project`, `paths`,
   `resource_key`, `resource_kind`, `NativeLifecycleRequest`, converted
   `ManagedLifecycleKind`, input, prior, `acknowledged`, filesystem,
   `json_limits`) and call `port.plan`.
6. Consume `plan.manifest`, `plan.current_fingerprint`, and
   `plan.desired_fingerprint` directly. Do not reconstruct MCP entries,
   omitted entries, current fingerprints, or desired fingerprints from target-
   native file writes.
7. Defense-in-depth: if `acknowledged == false` and the returned manifest
   contains any `ManagedProjection::Omitted`, block with the existing partial-
   operation acknowledgment error. Required-unsupported components surface as
   `ManagedProjectionError::RequiredUnsupported`, not as omissions.
8. Generalize `validate_managed_project_ownership` with a `target: &HarnessId`
   parameter replacing the two hardcoded Codex lookups; its drift, unowned,
   update-required, and pending-attempt recovery semantics otherwise stay the
   same.
9. Translate `plan.files` and `plan.trees` into the existing CLI-private
   execution-port entry types, derive operation surfaces from those writes,
   and build the managed-materialization operation for the resolved target.
10. For non-removal, seed target-local state from the resolved target,
    checkout/prior source and revision, `plan.desired_fingerprint`, and the
    returned manifest. For removal, keep `seed = None`. Existing state,
    pending/retry, publication, rollback, load verification, and rendering
    flows remain unchanged.

Implementation notes:

- `plan_managed_codex_project_lifecycle` and `ManagedCodexProjectPlanContext`
  are deleted. `ManagedProjectPlanContext` replaces the latter without adding
  `SourceRevisionResolver`; source checkout ownership stays in CLI through the
  existing `resolve_git_skill_source` path.
- `CodexManagedProjection` must disappear from CLI imports and call sites. The
  CLI reaches Codex only because the registry-selected adapter returns a port.
- The stale helpers implied by the old design (`observe_current_projection_
  fingerprint`, `plan_as_mcp`, CLI-side `managed_projection_manifest`
  reconstruction, `ManagedAcquisitionContext`, `AcquiredProjection`, and
  `plan.omitted`) are not part of Unit 3's implementation path.
- `NativeLifecycleKind` remains CLI-internal; the already-added
  `ManagedLifecycleKind` is populated by conversion at the port boundary.
- Shared error text in the orchestrator must be target-neutral; adapter-owned
  details continue to flow through `ManagedProjectionError::{code, summary}`.

Acceptance criteria:

- [ ] `plan_managed_codex_project_lifecycle` and
      `ManagedCodexProjectPlanContext` no longer exist; lifecycle dispatch calls
      `plan_managed_project_lifecycle` through `adapter.managed_projection()`.
- [ ] `git grep -n 'HarnessId::new("codex")' crates/cli/` and
      `git grep -n 'CodexManagedProjection' crates/cli/` return no matches.
- [ ] `git grep -n 'plan_as_mcp\|AcquiredProjection\|ManagedAcquisitionContext\|plan\.omitted' crates/cli/`
      returns no matches.
- [ ] `validate_managed_project_ownership` takes `target: &HarnessId` and
      preserves drift/unowned/update-required/pending-attempt-recovery
      semantics.
- [ ] Apply lifecycles resolve exactly one `ResolvedSourceCheckout` with no
      target-specific CLI side channel; remove lifecycles perform no source
      resolution.
- [ ] The orchestrator persists the adapter-returned manifest and validates
      directly against adapter-returned current/desired fingerprints.
- [ ] Defense-in-depth acknowledgment rejects returned `ManagedProjection::Omitted`
      entries when `acknowledged == false`.
- [ ] Existing Codex managed-project tests pass without assertion weakening
      beyond the approved source-free marketplace-removal behavior, and a
      temporary non-Codex fake port proves the current `plan` API can drive a
      planned operation/entry/seed through the shared path.

---

### Unit 4: Shared acceptance matrix and regression coverage

**Files**: `crates/test-support/src/harness_profile.rs` (modified — add a
managed-projection profile field); `crates/test-support/src/managed_acceptance.rs`
(new); `crates/cli/src/application/tests.rs` (modified — port the Codex
managed-project tests onto the shared matrix and add the fake-adapter test).

**Story**: `feature-managed-fallback-target-parity-acceptance`.

```rust
// crates/test-support/src/managed_acceptance.rs

/// The shared managed-projection acceptance matrix every
/// `ManagedProjectionPort` adapter must pass, mirroring the registry's
/// `acceptance_matrix` for native lifecycle. Covers: marketplace + plugin
/// acquisition, complete skill-tree projection, MCP merge, foreground
/// acknowledgment of optional omissions, required-unsupported blocking,
/// drift detection, unowned-destination rejection, update-required rejection,
/// pending-attempt recovery, effective-load verification (via the existing
/// LoadVerifier), and immediate-repeat idempotency.
pub fn managed_acceptance_matrix(
    profile: &ManagedProjectionProfile,
    machine: &IsolatedMachine,
) -> ManagedAcceptanceReport;

/// A fake managed-projection profile. Codex/Claude provide real ones; future
/// adapter features provide their own. Adding a managed-fallback target adds
/// a constructor here, not a new test branch.
#[derive(Clone, Debug)]
pub struct ManagedProjectionProfile {
    pub id: HarnessId,
    pub catalog_destinations: &'static [&'static str],
    pub mcp_destination: Option<&'static str>,
    pub skill_destination: &'static str,
    /// Drives a throwaway `ManagedProjectionPort::plan` impl that exercises
    /// the full target-agnostic path against this profile.
    pub port: fn() -> &'static dyn ManagedProjectionPort,
}
```

`FakeHarnessProfile` (from the registry feature) gains an optional
`managed_projection: Option<ManagedProjectionProfile>` so adapters that opt
into managed fallback get the full matrix; adapters that do not are skipped.

**Implementation Notes**:

- The matrix reuses the existing Codex managed-project tests
  (`managed_project_publication_failures_restore_then_retry_once_and_noop`,
  `managed_project_tree_limits_preserve_planning_and_revalidation_failures`,
  the pending-attempt recovery tests at `tests.rs:833-969`, and the
  ownership-validation tests at `tests.rs:1360-1506`) as its Codex instance.
  Those tests are ported onto the matrix's `ManagedProjectionProfile::codex()`
  without assertion changes.
- The fake-adapter test (Unit 3's temporary test, formalized) registers a
  throwaway `ManagedProjectionProfile` for a non-Codex `HarnessId` and
  asserts the orchestrator resolves an apply checkout, passes remove with no
  checkout, consumes returned manifest/current/desired fingerprint evidence
  directly, and drives ownership, drift, and idempotency through the port —
  proving the path is target-agnostic before any concrete adapter feature lands.
- Low-value tests are not added: no per-field test of `ManagedPluginWrite`
  round-trip beyond the port contract test (Unit 1), no snapshot of MCP TOML
  bytes (the existing Codex tests already pin the format), and no separate
  test of the `From` conversions beyond the orchestrator integration.

**Acceptance Criteria**:

- [ ] `managed_acceptance_matrix(&ManagedProjectionProfile::codex(), machine)`
      passes the full acquisition/projection/MCP/acknowledgment/drift/unowned/
      update-required/pending-recovery/verification/idempotency suite, with
      assertions byte-identical to today's Codex tests.
- [ ] A fake-adapter profile for a non-Codex `HarnessId` passes the same
      matrix through `ManagedProjectionPort::plan`, proving the orchestrator
      is target-agnostic, `Apply` receives exactly one `ResolvedSourceCheckout`,
      and `Remove` receives no checkout.
- [ ] `FakeHarnessProfile::codex().managed_projection` is `Some` and
      `FakeHarnessProfile::claude().managed_projection` matches Claude's
      managed-fallback opt-in (Claude's managed-project lifecycle state is
      preserved as-is).
- [ ] Immediate-repeat idempotency holds: running the matrix twice produces
      `OperationOutcome::NoChange` on the second pass with no duplicate
      artifacts or state entries.

---

## Implementation discovery and contract amendment

The Codex adapter story (`feature-managed-fallback-target-parity-codex-
adapter`) reached `stage:implementing`, probed the approved Unit 1 contract
against the actual Codex orchestrator, and stopped before any code change
because the contract could not carry the evidence required to relocate
Codex behavior faithfully. Its discovery is preserved verbatim in that
story's body; this section records the resulting contract amendment
without erasing the original Unit 1 design above.

### Evidence gaps in the approved contract

1. **No checked-out source root.** `ManagedAcquisitionContext::revision_
   resolver` is a `SourceRevisionResolver`, whose only operation returns a
   `ResolvedRevision` (a git commit id). It cannot return the checked-out,
   confined source root that `read_codex_catalog_at_root` and
   `read_complete_codex_plugin` must read. Recreating `resolve_git_skill_
   source` inside the adapter would duplicate the git process/cache
   boundary and contradict the design decision that acquisition reuses
   shared git resolution.
2. **No selected marketplace source for fresh plugin installs.** A fresh
   plugin install has no source on the plugin resource or
   `NativeLifecycleRequest`; the existing orchestrator resolves the
   selected marketplace source from inventory and passes it to catalog
   lookup. `ManagedAcquisitionContext` carried neither documents nor a
   typed marketplace source, so the adapter could not locate
   `plugin@marketplace` without a CLI-owned Codex side channel.
3. **No projection evidence.** `ManagedProjectionPlan` carried writes and
   omissions only. The Codex projection helper also produces the exact
   current aggregate fingerprint, desired aggregate fingerprint, and the
   complete `Vec<ManagedProjection>` manifest (Skill + per-server Mcp
   fingerprints + Omitted). Those drive ownership validation, drift,
   pending-attempt recovery, update-required checks, and persisted
   projection state. They cannot be reconstructed from
   `ManagedFileWrite` without parsing Codex TOML in CLI, and hashing the
   whole file would include unmanaged configuration and change semantics.
4. **Mandatory source acquisition for removal.** The acquire-then-project
   split forced every lifecycle through acquisition, but plugin removal
   plans from the prior manifest without source, and marketplace removal
   only needs current filesystem observation of the catalog destination.

### Amended contract (story `feature-managed-fallback-target-parity-
contract-evidence`)

The amendment does not reopen the approved Unit 1 story (it stays `done` at
its commit). It adds one new child story at `stage:implementing`, parented
here, that revises the public contract types in place:

- **Caller-resolved checkout.** New core type `ResolvedSourceCheckout {
   root, source, revision }`. The orchestrator resolves it once using the
   existing `resolve_git_skill_source` machinery and hands it to the
   adapter via the input enum. `SourceRevisionResolver` stays
   revision-only and leaves the managed-projection context.
- **One authoritative source per checkout.** For a fresh plugin install, the
   orchestrator resolves the selected marketplace source into
   `ResolvedSourceCheckout`. The adapter reads from `checkout.root()` and uses
   `checkout.source()` as provenance. `Apply` carries no second source field
   that could disagree with the checkout.
- **Complete projection evidence.** `ManagedProjectionPlan` gains
   `manifest: Vec<ManagedProjection>`, `current_fingerprint:
   Option<Fingerprint>`, and `desired_fingerprint: Option<Fingerprint>`.
   The adapter produces them directly; the orchestrator never reconstructs
   them from writes. The `omitted: Vec<OmittedComponent>` field and the
   `OmittedComponent` type are removed (omissions live exclusively as
   `ManagedProjection::Omitted` inside `manifest`, so the two cannot
   diverge).
- **Removal without source.** `ManagedProjectionInput::Remove` carries no
   checkout, so removal cannot accidentally require source. Marketplace
   removal no longer fails with `managed_project_source_missing` when the
   source is absent but the catalog projection is observable; plugin
   removal is unchanged.
- **One `plan` method.** `acquire`/`project` collapse into a single
   `ManagedProjectionPort::plan` taking `ManagedProjectionContext` whose
   `input: ManagedProjectionInput` distinguishes apply from remove.
   `AcquiredProjection` and `ManagedAcquisitionContext` no longer cross
   the boundary. This is the shortest type-safe contract: invalid states
   (removal carrying a checkout; apply without one) are unrepresentable.

### Revised interface summary

```rust
// skilltap-core
pub struct ResolvedSourceCheckout { /* root, source, revision */ }

pub struct ManagedProjectionPlan {
    pub trees: Vec<ManagedPluginWrite>,
    pub files: Vec<ManagedFileWrite>,
    pub manifest: Vec<ManagedProjection>,
    pub current_fingerprint: Option<Fingerprint>,
    pub desired_fingerprint: Option<Fingerprint>,
}
// Removed: OmittedComponent, AcquiredProjection, ManagedProjectionPlan::omitted.

// skilltap-harnesses
pub enum ManagedProjectionInput<'a> {
    Apply { checkout: &'a ResolvedSourceCheckout },
    Remove,
}

pub struct ManagedProjectionContext<'a> { /* target, project, paths,
    resource_key, resource_kind, request, kind, input, prior, acknowledged,
    filesystem, json_limits */ }

pub trait ManagedProjectionPort: Sync {
    fn plan(&self, context: &ManagedProjectionContext<'_>)
        -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}
// Removed: ManagedAcquisitionContext, acquire, project.
```

The `HarnessAdapter::managed_projection() -> Option<&dyn
ManagedProjectionPort>` accessor, the `ManagedLifecycleKind` enum,
`ManagedPluginWrite`, `ManagedFileWrite`, and the entire
`ManagedProjectionError` model (codes, summaries, `Other` discipline) are
unchanged.

### Removal behavior change

Modeling removal as `Remove` (no checkout) is an intentional, contract-
driven behavior change: marketplace removal no longer requires source
acquisition. Plugin removal was already source-free. The Codex regression
suite is updated (not weakened) to reflect this; the orchestrator and
acceptance stories inherit the amended port shape when they implement.

## Implementation Order

1. `feature-managed-fallback-target-parity-contract` (Unit 1, port + core
   types + adapter accessor) — `depends_on: []`. Foundation story;
   terminalized at `caf5df03`.
2. `feature-managed-fallback-target-parity-contract-evidence` (Unit 1
   amendment) — `depends_on: [feature-managed-fallback-target-parity-
   contract]`. Revises the public contract types in place to carry the
   four pieces of evidence the Codex relocation requires: caller-resolved
   checkout with one authoritative source, complete projection evidence, and
   removal-without-source. Collapses acquire/project into one `plan`
   method. Does not touch production Codex behavior.
3. `feature-managed-fallback-target-parity-codex-adapter` (Unit 2, Codex
   relocation onto the amended port) — `depends_on:
   [feature-managed-fallback-target-parity-contract-evidence]`.
   Behavior-preserving relocation for install/update; removal corrected to
   drop mandatory source. The CLI orchestrator still drives Codex through
   the relocated helpers until Unit 4 flips the dispatch.
4. `feature-managed-fallback-target-parity-orchestrator` (Unit 3,
   target-agnostic orchestrator + generalized ownership validation +
   dispatch flip) — `depends_on: [feature-managed-fallback-target-parity-
   contract, feature-managed-fallback-target-parity-codex-adapter]`. The
   composition boundary that makes the lifecycle target-agnostic.
   Transitively depends on the contract-evidence amendment through the
   codex-adapter; consumes the amended port surface directly.
5. `feature-managed-fallback-target-parity-acceptance` (Unit 4, shared
   matrix + regression + fake-adapter proof) — `depends_on:
   [feature-managed-fallback-target-parity-contract,
   feature-managed-fallback-target-parity-codex-adapter,
   feature-managed-fallback-target-parity-orchestrator]`. Reusable
   contract adapter features will invoke; Codex regression pinned.

The parent feature `feature-managed-fallback-target-parity` carries the
design body only; it has no inline stride. Its child stories carry the
units above. Each child's `depends_on` points only at sibling stories (or
is empty) — never at the parent feature id — so the graph is executable:
the foundation story terminalized first, then contract-evidence, then
codex-adapter, then orchestrator, then acceptance. The orchestrator and
acceptance stories depend on the contract-evidence amendment transitively
through the codex-adapter (their direct edge to it is implied, keeping the
graph minimal); both consume the amended port surface directly. The parent
feature terminalizes once all children are done, which also unblocks the
four sibling adapter features that legitimately wait on the whole
managed-fallback deliverable.

## Elimination

- **Eliminate** `plan_managed_codex_project_lifecycle` and its
  `ManagedCodexProjectPlanContext` — replaced by target-agnostic
  `plan_managed_project_lifecycle` + `ManagedProjectPlanContext`.
- **Eliminate** the two `HarnessId::new("codex").expect(...)` literals in
  `validate_managed_project_ownership` — generalized to a `target: &HarnessId`
  parameter.
- **Eliminate** the Codex-specific helpers from CLI:
  `resolve_codex_marketplace_source`, `read_codex_catalog_at_root`,
  `plan_codex_component_projections`, `plan_codex_mcp_config`,
  `mcp_depends_on_plugin_root`, `read_complete_codex_plugin`,
  `ResolvedCodexMarketplace`, `CodexComponentProjectionPlan`,
  `CodexMcpConfigPlan` — relocated into the `CodexManagedProjection` adapter
  and its private helpers in harnesses.
- **Eliminate** the intermediate `CodexComponentProjectionPlan` /
  `CodexMcpConfigPlan` types entirely — their fields fold into
  `ManagedProjectionPlan { trees, files, omitted }`.
- **Relocate** (not duplicate) the target-agnostic helpers
  (`observe_managed_project_tree`,
  `managed_project_tree_observation_limits`, `managed_projection_manifest`,
  `append_projection_tree`, `managed_project_error`, `previously_applied`)
  unchanged into the shared orchestrator; they were already target-agnostic.
- **Retain intentionally**: the Codex catalog destination order, MCP
  destination, MCP TOML table name, and plugin-root-relative evidence code as
  adapter-private constants in `CodexManagedProjection`; the existing
  `ManagedCodexCatalog` codec in harnesses; the `ManagedProjection` /
  `PendingManagedAttempt` / `TargetResourceState` state shape and
  `STATE_SCHEMA_VERSION`; the `ManagedProjectLifecyclePort` execution/revalid-
  ation boundary and its CLI-private entry types; the publication boundary
  (`PublicationBatch`/`PublicationSink`/`LoadVerifier`/
  `record_verified_publication`/`ManagedArtifactRepository`); the
  `managed_project_lifecycle() -> bool` gate.

No separate `[refactor]` / `[cleanup]` child story is warranted: every
elimination is bound to the unit that introduces its replacement (Unit 2
relocates the Codex helpers; Unit 3 deletes the Codex orchestrator and
generalizes ownership validation), and each is independently reviewable as
part of that story.

## Testing

- **Port contract test (Unit 1)**: a throwaway `ManagedProjectionPort` impl
  round-trips `AcquiredProjection` through `acquire` and `project` and the
  returned `ManagedProjectionPlan` equals the inputs. Protects the
  object-safety and cross-crate currency contract.
- **Codex relocation regression (Unit 2)**: every existing Codex
  managed-project test in `crates/cli/src/application/tests.rs` passes
  without assertion changes after the helpers relocate into
  `CodexManagedProjection`. Protects the no-behavior-change guarantee.
- **Target-agnostic orchestrator test (Unit 3)**: a throwaway
  `ManagedProjectionPort` registered for a fake `HarnessId` produces a
  planned operation/entry/seed driven entirely through the port; the
  ownership-validation generalization preserves drift/unowned/update-required
  semantics for both Codex and the fake target. Protects the
  no-Codex-literal-in-CLI invariant.
- **Shared acceptance matrix (Unit 4)**:
  `managed_acceptance_matrix(&ManagedProjectionProfile::codex(), machine)`
  pins the full Codex behavior; the fake-adapter profile proves
  target-agnosticism; immediate-repeat idempotency holds. Protects the
  reusable contract every sibling adapter feature will rely on.
- **Removals**: assertions previously pinning `plan_managed_codex_project_
  lifecycle`, `ManagedCodexProjectPlanContext`, the `Codex*Plan` intermediate
  types, or the hardcoded `"codex"` ownership lookup are updated in the same
  story that removes the symbol; no tautological replacement is introduced.

Low-value tests are not added: no per-field serialization test for
`ResolvedSourceCheckout` or `ManagedProjectionPlan` (they are planning
currency, not serialized), no exhaustive `ManagedProjectionError` code table
beyond what the orchestrator surfacing already exercises, and no separate test
of the `From` conversions beyond the orchestrator integration.

## Pre-mortem

- **Riskiest assumption**: that the single `ManagedProjectionPort::plan`
  boundary cleanly separates target-specific projection from target-agnostic
  orchestration without leaking Codex shape. *Failure mode*: a future adapter
  (e.g., a JSONC-MCP target) cannot implement the port without working around
  Codex-shaped fields. *Mitigation*: the port exchanges only normalized
  inputs/evidence (`ManagedProjectionInput`, `ManagedProjectionContext`, and
  `ManagedProjectionPlan`); MCP document bytes are opaque
  (`ManagedFileWrite::desired: Option<Vec<u8>>`) and the orchestrator never
  parses them; skill destinations reuse the existing `SkillProjectionPort`, so
  no path logic crosses the boundary. The Unit 4 fake-adapter test is the
  canary: if it cannot exercise the full matrix through a non-Codex profile,
  the abstraction has leaked.
- **Migration ordering hazard**: Unit 2 (relocate Codex helpers) and Unit 3
  (flip the dispatch) could be merged into one large change, risking a
  behavior regression in the same commit that changes the call path.
  *Mitigation*: the two units are deliberately split. Unit 2 is pure
  relocation with the CLI orchestrator still in command (helpers move to
  harnesses, CLI imports them); Unit 3 then flips the dispatch with the
  helpers already in their final home. Each commit is independently
  compilable and testable, and each preserves the full Codex test suite.
- **`NativeLifecycleKind` lift breaks the CLI enum**: lifting the kind into
  harnesses could desynchronize the CLI's kind from the port's kind.
  *Mitigation*: the CLI keeps `NativeLifecycleKind` as a local alias with a
  `From<NativeLifecycleKind> for ManagedLifecycleKind` conversion; the port
  only ever sees the shared kind. The existing lifecycle tests pin the
  operation-id labels, which are unaffected because `lifecycle_operation_id`
  still formats the CLI kind.
- **Foreground acknowledgment regression**: the
  `partial_operation_requires_acknowledgment` gate must fire identically
  after the port owns projection. *Failure mode*: the adapter lists an
  optional omission in `plan.manifest` when `acknowledged` is false, and the
  orchestrator accepts it. *Mitigation*: the port contract specifies optional
  omissions appear as `ManagedProjection::Omitted` manifest entries only when
  `context.acknowledged` is true; the orchestrator additionally re-checks that
  any `Omitted` entry with `acknowledged == false` blocks with
  `partial_operation_requires_acknowledgment` before producing a plan. The
  existing MCP acknowledgment test pins this for Codex; the Unit 4 matrix pins
  it for any adapter.
- **Required-unsupported leakage**: a target that cannot represent a
  required component must block even with `--yes`. *Failure mode*: the
  adapter returns a plan omitting a required component. *Mitigation*: the
  port contract requires required-unsupported to surface as
  `ManagedProjectionError::RequiredUnsupported` (mapped to
  `managed_project_unsupported_required`), never as an `Omitted` entry. The
  orchestrator treats any `Err` from `plan` as a block. The Unit 4 matrix
  includes a required-unsupported fixture.
- **Marketplace-kind drift for non-Codex targets**: a future adapter that
  does not support `ResourceKind::Marketplace` could be asked to project one
  if the inventory is misconfigured. *Failure mode*: silent wrong behavior.
  *Mitigation*: the port's `plan` returns
  `ManagedProjectionError::UnsupportedResourceKind` for kinds the adapter
  does not handle, which the orchestrator surfaces as a typed attention
  result. The orchestrator never assumes a target supports marketplace
  projection.
- **Effective-load verification gap**: the publication boundary already
  requires fresh effective observation via `LoadVerifier`. *Failure mode*: a
  managed-fallback target whose documented load path cannot be observed.
  *Mitigation*: unchanged from today — the target stays pending/blocked and
  no ownership record is written; skilltap does not infer success from files
  on disk. This feature does not weaken that boundary; it only feeds it via
  the adapter's projection writes.

## Risks

- **Port-trait sprawl**: `ManagedProjectionPort` plus the existing
  `NativeLifecycleVector` / `InstructionBridgePort` / `SkillProjectionPort`
  is the fourth optional port on `HarnessAdapter`. *Mitigation*: the port
  is behind `Option<&dyn Port>` defaulting to `None`, so a file-managed
  adapter implements only `managed_projection` (+ the required detection/
  observation/profile core). If a fifth port appears, revisit splitting
  `HarnessAdapter` into a required-core trait plus a capability-probe trait;
  deferred until earned (same mitigation the registry feature recorded).
- **`3.0.0` in quality gate**: this feature targets `main` after `3.0.0`
  ships. The state shape is unchanged (`STATE_SCHEMA_VERSION` stays), so the
  change is additive and must not be cherry-picked onto the release branch.
  Story 3 (orchestrator flip) calls this out in its body.
- **Two filesystem traits in the port context**: `ManagedAcquisitionContext`
  carries `&dyn ConfinedFileSystem` while the CLI execution port composes
  `FileSystem + DirectoryTreeFileSystem + ConfinedFileSystem`. *Mitigation*:
  the port only needs `ConfinedFileSystem` for catalog/tree reads; the
  execution port's broader bound stays CLI-side. The orchestrator adapts
  between them (its `managed_project_filesystem()` already satisfies both).
- **Adapter-owned MCP codec correctness is now opaque to the orchestrator**:
  the orchestrator cannot sanity-check adapter-produced MCP bytes.
  *Mitigation*: the `LoadVerifier` boundary still requires the target to
  load the projected resource effectively; a malformed MCP document fails
  verification and blocks ownership. The Codex regression tests pin the
  specific TOML shape; future adapters pin theirs in their own features.

## Implementation outcome

All five child stories are terminal. The delivered implementation includes the
base adapter port, the evidence/source amendment discovered during Codex
relocation, the Codex adapter, the target-neutral CLI orchestrator, and a
reusable dependency-neutral acceptance matrix exercised through real lifecycle
dispatch for Codex and a non-Codex adapter. Install/update behavior remains
regression-pinned; marketplace removal is intentionally source-free; state
schema and publication boundaries are unchanged. The aggregate review should
treat the earlier acquire/project descriptions and migration sketches as
superseded by the amendment and completed child records.

## Cross-feature impact

- Unblocks `epic-expanded-harness-support-file-managed`,
  `-native-coexistence`, `-configuration-constrained`,
  `-trust-interactive`, and `-pi` at the dependency-graph level (all
  `depends_on: [epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`). Each consumes
  `ManagedProjectionPort` to supply its target-specific acquisition and
  projection without reinventing ownership/drift/idempotency.
- Does **not** unblock `epic-expanded-harness-support-candidate-admission`
  beyond what the registry already unblocked; candidate admission depends on
  concrete adapter evidence, not on this shared port.
- Does **not** change `epic-cross-harness-materialization-publish` (the
  publication boundary is reused verbatim) or any released 3.0.0 work.
