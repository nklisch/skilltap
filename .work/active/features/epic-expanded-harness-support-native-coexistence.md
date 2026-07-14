---
id: epic-expanded-harness-support-native-coexistence
kind: feature
stage: implementing
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-14
---

# Native-Coexistence Adapters for Droid, Qwen, and Copilot

## Brief

Deliver complete adapters for Factory Droid, Qwen Code, and GitHub Copilot CLI.
These targets combine documented skill and MCP files with native plugin,
extension, marketplace, conversion, or policy behavior that must coexist with
skilltap-managed fallback rather than being flattened into it.

For every resource, prefer and independently track a faithful native
distribution when one exists; use managed component projection only for the
target lacking a native equivalent. Each adapter owns its native identity,
scope, precedence, enterprise or trust constraints, structured observation,
and verified version profiles while sharing target-neutral execution and state
machinery. The feature includes isolated native validation and complete
acceptance-contract evidence for all three targets.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Consolidate native-versus-managed selection into capability-driven adapter
  composition instead of accumulating target-name branches in application code.

## Foundation references

- `docs/VISION.md` — Native First, Faithfulness Before Portability.
- `docs/SPEC.md` — Plugin Lifecycle, Marketplace Lifecycle, Ownership and Removal.
- `docs/HARNESS-CONTRACTS.md` — Contract Rules, Expanded Target Set.

## Grounding summary

The design was mapped by direct reading only because the caller prohibited
nested agents and peer review. The relevant delivered foundations are:

- `TargetRegistry`, `HarnessAdapter`, `NativeLifecycleVector`,
  `SkillProjectionPort`, and `ManagedProjectionPort` in
  `crates/harnesses/src/registry.rs` and
  `crates/harnesses/src/managed_projection.rs`. The registry is the sole target
  list; new adapters require one module plus one canonical registry entry.
- The generic managed lifecycle in `crates/cli/src/application.rs` and
  `crates/cli/src/application/lifecycle.rs` preserves target-local
  `Provenance`, `Ownership`, native ids, projection manifests, revisions, and
  operation journals, but currently enters managed projection before checking
  native capability whenever `managed_project_lifecycle()` is true. That is
  correct for Codex's unsupported project-native lifecycle and wrong for these
  three coexistence targets, all of which have native project lifecycles.
- `ManagedProjectionPort` is target-specific and already keeps source parsing,
  native MCP codecs, exact write paths, and omission evidence out of core. It is
  currently project-shaped (`ManagedProjectionContext::project` and
  `plan_managed_project_lifecycle`) even though all three new adapters must
  support managed fallback at global and project scope.
- `TargetResourceState` already distinguishes native/adopted harness ownership
  from materialized skilltap ownership. No state schema or universal plugin
  identity is needed: the applied target binding is the representation pin.
- The review-ready project skill contract validates one canonical
  `<project>/.agents/skills/<name>` tree and obtains each native project root
  through `SkillProjectionPort`. Droid (`.factory/skills`) and Qwen
  (`.qwen/skills`) therefore receive relative per-skill links; Copilot consumes
  `.agents/skills` directly and receives no redundant project operation.
- `NativeLifecyclePort` already uses resolved executable identity, bounded
  direct argument vectors, exact scope working directories, revalidation under
  lock, post-observation, and target-local state refresh. Each new lifecycle
  vector must reuse it rather than invoke `Command` or edit a native cache.
- `FakeHarnessProfile`, `acceptance_matrix`, `ManagedProjectionProfile`, and
  `managed_acceptance_matrix` provide the reusable test contracts. The new
  profiles must extend these fixtures rather than add a second target table.

Source-direct attestations establish distinct contracts:

- Factory: `droid plugin` provides scoped native marketplace/plugin lifecycle;
  plugins can contain skills, commands, agents, hooks, and MCP, Claude plugin
  interoperability is native behavior, updates follow the latest marketplace
  commit without pins, and cache files are read-only evidence. Skills load from
  `~/.factory/skills` / `.factory/skills`; MCP uses
  `~/.factory/mcp.json` / `.factory/mcp.json`, user entries win name collisions,
  and changes auto-reload.
- Qwen: `qwen extensions` provides scoped sources and extension lifecycle and
  performs target-owned Claude/Gemini conversion. Skills load from
  `~/.qwen/skills` / `.qwen/skills`; MCP is the `mcpServers` member of scoped
  `settings.json`, supports stdio/HTTP/SSE, and requires fresh-session
  verification after changes.
- Copilot: native marketplace/plugin lifecycle accepts Copilot and documented
  Claude marketplace forms. Skills load from several roots including canonical
  personal/project `.agents/skills`. MCP uses user
  `~/.copilot/mcp-config.json` and repository `.mcp.json` or
  `.github/mcp.json`; `copilot mcp list|get --json` supplies effective evidence.
  Workspace definitions outrank user definitions, while enterprise allowlists
  and repository trust may narrow effective native state.

The research substrate does not attest exact current executable versions or all
native argv/list JSON shapes. None of `droid`, `qwen`, or `copilot` is installed
in the current isolated environment. The implementation therefore must refresh
official command references and validate clean native binaries before adding an
exact compiled profile. Guessed versions or command shapes are explicitly
forbidden; an adapter may be registered observe-only until its first exact
profile is proven within its story.

Foundation documents already describe these targets and the intended native-
first/managed-fallback behavior. This is code-first work against future-state
foundation truth; implementation should update a foundation assertion only if
native validation disproves it, not merely to add inventory-like coverage.

## Design decisions

- **Applied representation is pinned per target binding:** native/adopted
  harness-owned state continues through native update/removal; materialized
  skilltap-owned state with a projection manifest continues through managed
  update/removal. Reconciliation never silently migrates an existing binding
  merely because another representation later becomes available.
- **Fresh marketplace registration chooses from evidence, not target names:** an
  adapter-private distribution assessor reads one caller-resolved checkout,
  parses only the target's documented native catalog/conversion inputs, and
  compares that native component plan with the managed component plan. A
  faithful native distribution wins. Managed wins only when native is absent or
  a verified managed plan preserves a strict superset. Incomparable partial
  plans block with their exact component consequences rather than relying on an
  invented fidelity score.
- **Plugin representation follows its marketplace:** a fresh
  `plugin@marketplace` install uses the exact target-local representation of
  its registered marketplace. This prevents a native marketplace from spawning
  an unrelated managed plugin, and prevents a managed marketplace from being
  passed to a native CLI that cannot resolve it.
- **Unknown native authority does not trigger fallback:** if a source has a
  target-native equivalent but the executable version is unknown or its probe
  narrows mutation support, the operation remains blocked/observe-only. Managed
  projection is not a loophole around an unverified native lifecycle.
- **No new representation field in state:** the existing combination of
  `Provenance`, `Ownership`, native id, and managed projection manifest is the
  source of truth. Contradictory evidence is a typed conflict, not guessed into
  a route.
- **Managed projection becomes concrete-scope, not project-only:** generalize
  the existing planner/port to receive `&Scope`; target adapters derive their
  documented global or project roots. Codex keeps its current global-native and
  project-managed behavior through the same selector, while the three new
  adapters can use managed fallback at either scope when no faithful native
  distribution exists.
- **Normalized component graphs are not a universal plugin format:** existing
  `SourceComponentGraph` and `MaterializationPlan` carry only target-neutral
  component identities, dependencies, and requiredness for comparison. Each
  source reader still parses its own manifest/catalog and each target adapter
  still owns native conversion semantics, MCP codec, paths, precedence, and
  reload behavior.
- **Native conversion must prove component fidelity:** Factory's Claude
  interoperability and Qwen's Claude/Gemini conversion count as native only for
  concrete components validated by that adapter's conversion matrix and
  effective observation. A similar directory shape or a successful install
  exit does not prove hooks, agents, or commands faithful.
- **Precedence is health evidence, not drift rewriting:** Factory's user-over-
  project MCP collision and Copilot's workspace/user/plugin layering can make a
  correctly declared managed entry ineffective. Status reports the effective
  blocker and leaves both native documents untouched; it does not rewrite the
  higher-precedence unmanaged entry.
- **Project skills consume the shared link contract:** adapters provide only
  their native roots and compatibility evidence. They do not copy project
  standalone skills, reconcile links themselves, or special-case target ids in
  CLI.
- **Global skills retain target-local publication:** the project link contract
  is intentionally project-only. Global standalone skills use each adapter's
  documented user root through the existing global lifecycle; Copilot may
  consume canonical `~/.agents/skills`, while Droid/Qwen use their native roots.
- **No instruction bridge is added:** instruction support is not an admission
  requirement. Copilot's documented `AGENTS.md` behavior may be observed, but
  this feature does not expand the shared instruction lifecycle for any of the
  three targets.
- **No UI work:** this is a non-interactive CLI/domain/adapter feature; no mockup
  surface exists.

## Architectural choice

**Chosen — evidence-routed coexistence over the existing native and managed
ports.** Add a small pure lifecycle-representation selector in core and an
adapter-owned `NativeDistributionPort` beside the existing optional registry
ports. The CLI resolves a source checkout once, obtains native and managed
component plans, asks the pure selector for one representation, and then uses
exactly the existing revalidated native or managed executor. Existing state
pins later operations. Managed projection is generalized from project to exact
scope, but native codecs and source readers remain adapter-private.

**Rejected — set `managed_project_lifecycle = true` on all three adapters.** The
current branch would always materialize project resources and bypass verified
native project lifecycle. That violates Native First and loses native update,
identity, conversion, consent, and enablement behavior.

**Rejected — always call native lifecycle and fall back after failure.** A
post-mutation fallback cannot be planned safely, conflates command failure with
absence of a native equivalent, and risks two installations with ambiguous
ownership. Selection must happen before mutation from source/profile evidence.

**Rejected — create a common Droid/Qwen/Copilot plugin manifest.** No such
native contract exists. A universal manifest would erase Factory commit update
semantics, Qwen conversion evidence, Copilot policy/trust state, and each
harness's marketplace identity.

## Implementation Units

### Unit 1: Coexistence routing and concrete-scope managed contract

**Files**:

- `crates/core/src/lifecycle_representation.rs` (new) and
  `crates/core/src/lib.rs` — pure route types and representation selection.
- `crates/harnesses/src/native_distribution.rs` (new),
  `crates/harnesses/src/registry.rs`, and `crates/harnesses/src/lib.rs` —
  adapter-owned native distribution assessment port.
- `crates/harnesses/src/managed_projection.rs` and
  `crates/harnesses/src/adapters/codex_managed.rs` — change the managed port
  from project-only context to exact concrete scope without changing Codex
  behavior.
- `crates/cli/src/application.rs`,
  `crates/cli/src/application/lifecycle.rs`, and
  `crates/cli/src/application/execution.rs` — one route selector before the
  existing native/managed planners; generalize managed naming and scope.
- `crates/cli/src/application/tests.rs` — representation pinning, preference,
  ambiguity, and Codex regression tests.

**Story**: `epic-expanded-harness-support-native-coexistence-contract`

```rust
// skilltap-core
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentation {
    Native,
    Managed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepresentationCandidate {
    pub representation: LifecycleRepresentation,
    pub plan: MaterializationPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepresentationEvidence {
    Existing(LifecycleRepresentation),
    Marketplace(LifecycleRepresentation),
    Fresh {
        native: Option<RepresentationCandidate>,
        managed: Option<RepresentationCandidate>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentationError {
    ContradictoryAppliedState,
    MissingMarketplaceRepresentation,
    RequiredComponentsBlocked,
    IncomparablePartialRepresentations,
    NoSupportedRepresentation,
}

pub fn select_lifecycle_representation(
    evidence: RepresentationEvidence,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;

pub fn applied_lifecycle_representation(
    state: &TargetResourceState,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;
```

```rust
// skilltap-harnesses
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDistributionAssessment {
    pub graph: SourceComponentGraph,
    pub plan: MaterializationPlan,
}

pub struct NativeDistributionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub checkout: &'a ResolvedSourceCheckout,
    pub requested_revision: Option<&'a RequestedRevision>,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

pub trait NativeDistributionPort: Sync {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}

pub trait HarnessAdapter: Sync {
    // existing methods
    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort> {
        None
    }
}
```

```rust
// amended managed port
pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    pub resource_key: &'a ResourceKey,
    pub resource_kind: ResourceKind,
    pub request: &'a NativeLifecycleRequest,
    pub kind: ManagedLifecycleKind,
    pub input: ManagedProjectionInput<'a>,
    pub prior: &'a [ManagedProjection],
    pub acknowledged: bool,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

fn plan_managed_lifecycle(
    registry: &TargetRegistry,
    target: &HarnessId,
    kind: NativeLifecycleKind,
    request: &NativeLifecycleSpec,
    resource: &DesiredResource,
    context: ManagedPlanContext<'_>,
) -> Result<PlannedManagedLifecycle, ErrorDetail>;
```

**Implementation notes**:

- `applied_lifecycle_representation` accepts harness-owned native/adopted
  bindings as native and skilltap-owned materialized bindings with a nonempty
  projection manifest as managed. Crossed ownership/provenance or both native
  and managed identity evidence return `ContradictoryAppliedState`.
- For install, the target-local marketplace binding supplies
  `RepresentationEvidence::Marketplace`. For update/removal, the resource's
  own binding supplies `Existing`. Only a fresh marketplace add compares
  candidates.
- A candidate with blocked required components is ineligible. Faithful native
  wins; faithful managed wins only when native is absent/partial. Between
  partial plans, one wins only when its included set is a strict superset and
  it does not add blocked required components. Equal partial plans prefer
  native. Incomparable sets block and retain both consequence lists.
- Native source assessment occurs before profile execution, but a selected
  native representation still requires the exact compiled capability. Unknown
  versions and narrowed probes block; they never cause reselection to managed.
- Resolve one checkout through the existing bounded source resolver. Both
  assessors borrow it; neither reclones, searches, or mutates the source.
- Replace the early `managed_project_lifecycle()` branch with route selection.
  Keep one composite execution plan and the existing foreign-operation proof;
  do not create a second executor.
- `ManagedProjectionContext::scope` allows adapters to derive global or project
  roots. Codex returns unsupported for global managed planning and retains
  native global routing; its project fixtures and bytes remain unchanged.
- No state schema changes. The route is recomputed from source/prior evidence
  and persisted through the existing target binding produced by the chosen
  executor.

**Acceptance criteria**:

- [ ] Existing native and managed bindings remain on their applied
      representation for update/removal; contradictory ownership fails closed.
- [ ] A plugin install follows the exact target-local marketplace
      representation, including when sibling targets use a different route.
- [ ] Faithful native beats managed; managed is selected only when it is the
      sole faithful candidate or a strict fidelity improvement; incomparable
      partial plans block with exact components.
- [ ] Unknown native versions never gain managed fallback merely because native
      mutation is unavailable.
- [ ] Managed projection plans both global and project scope through adapter
      paths while Codex's existing global-native/project-managed behavior and
      tests remain unchanged.
- [ ] No target id string branch is added to core or CLI, and no native cache is
      used as a write API.

---

### Unit 2: Factory Droid adapter

**Files**:

- `crates/harnesses/src/adapters/factory.rs` (new) — detection, exact profile,
  native argv, roots, observation, skill compatibility, and distribution
  assessment.
- `crates/harnesses/src/adapters/factory_managed.rs` (new) — Factory skill/MCP
  managed projection and JSON codec.
- `crates/harnesses/src/adapters/mod.rs`, `crates/harnesses/src/registry.rs`, and
  `crates/harnesses/src/lib.rs` — exports and one canonical registry entry.
- `crates/harnesses/tests/detection.rs`,
  `crates/harnesses/tests/lifecycle_scope.rs`, and
  `crates/harnesses/tests/normalization.rs` — source-direct adapter contracts.
- `crates/test-support/src/harness_profile.rs` — `FakeHarnessProfile::droid()`.

**Story**: `epic-expanded-harness-support-native-coexistence-factory`

```rust
pub struct FactoryAdapter;
pub struct FactoryLifecycle;
pub struct FactorySkillProjection;
pub struct FactoryNativeDistribution;
pub struct FactoryManagedProjection;

impl HarnessAdapter for FactoryAdapter {
    fn identity(&self) -> TargetIdentity; // id=droid, display=Factory Droid
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version_with_limits(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;
    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector>;
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort>;
    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort>;
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort>;
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath>;
}
```

**Implementation notes**:

- Refresh the official Factory CLI reference and validate a clean binary before
  committing `FACTORY_VERIFIED_PROFILES`. Pin exact version output, scoped
  marketplace/plugin argv, structured list output or version-gated bounded
  parser, project working directory, and postconditions. Unknown versions are
  observe-only.
- `FactoryLifecycle` uses `droid plugin ... --scope user|project` exactly as
  refreshed and always runs through `NativeLifecyclePort`. Native installed
  plugin content/cache is observed for identity/version only and never edited.
- Native assessment recognizes only Factory's attested native marketplace and
  documented Claude-compatible forms. A requested pin is not native-faithful
  because Factory updates track the latest marketplace commit; use managed
  projection when it can preserve the pin, otherwise block.
- `SkillProjectionPort` returns `~/.factory/skills` globally and
  `<project>/.factory/skills` for project scope. The shared project skill
  service creates per-skill relative links from `.factory/skills` to canonical
  `.agents/skills`; the adapter never creates them itself.
- Managed MCP writes preserve unknown JSON and unrelated servers in
  `mcpServers`. User scope targets `~/.factory/mcp.json`; project scope targets
  `.factory/mcp.json`. Removal deletes only target-state-proven owned entries
  and leaves an otherwise-empty user-authored document intact unless the whole
  file is proven skilltap-owned.
- Effective observation keeps native plugin resources and managed component
  projections as separate layers. A same-name user MCP server shadowing a
  managed project server yields a precedence health finding rather than a
  rewrite. Auto-reload is verified with a fresh structured native observation.
- Factory-only commands, agents, hooks, and other components stay native when
  the source distribution is faithful. Managed projection uses the existing
  materialization planner: required unsupported components block and optional
  omissions require foreground acknowledgment.

**Acceptance criteria**:

- [ ] `droid` is registry-derived in help/config/dispatch and is not added to
      first-party skilltap plugin bootstrap.
- [ ] One exact validated profile grants only attested scoped capabilities;
      unknown versions and narrowed probes are observe-only.
- [ ] Native marketplace/plugin install, update, remove, and immediate repeat
      preserve Factory identity, latest-commit semantics, scope, and cache
      ownership.
- [ ] Standalone complete skills work at both scopes; project scope uses the
      shared relative-link contract.
- [ ] Managed global/project MCP preserves unknown fields, reports user-over-
      project shadowing, reloads effectively, and removes only owned entries.
- [ ] A pinned source never routes to Factory's unpinned native update path.

---

### Unit 3: Qwen Code adapter

**Files**:

- `crates/harnesses/src/adapters/qwen.rs` (new) — detection/profile, extension
  lifecycle, source/conversion assessment, paths, observation, restart probe.
- `crates/harnesses/src/adapters/qwen_managed.rs` (new) — complete skill and
  scoped `settings.json` MCP projection.
- `crates/harnesses/src/adapters/mod.rs`, `crates/harnesses/src/registry.rs`, and
  `crates/harnesses/src/lib.rs` — exports and canonical registry entry.
- `crates/harnesses/tests/detection.rs`,
  `crates/harnesses/tests/lifecycle_scope.rs`, and
  `crates/harnesses/tests/normalization.rs` — target contract tests.
- `crates/test-support/src/harness_profile.rs` — `FakeHarnessProfile::qwen()`.

**Story**: `epic-expanded-harness-support-native-coexistence-qwen`

```rust
pub struct QwenAdapter;
pub struct QwenLifecycle;
pub struct QwenSkillProjection;
pub struct QwenNativeDistribution;
pub struct QwenManagedProjection;

impl NativeDistributionPort for QwenNativeDistribution {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}

impl ManagedProjectionPort for QwenManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}
```

**Implementation notes**:

- Refresh and validate the current `qwen` binary before adding exact profile
  constants. Pin `extensions sources` plus extension install/update/uninstall
  argv, global/project scope encoding (including workspace alias handling),
  version output, list observation, and fresh-session MCP probe.
- Native distribution assessment parses Claude, Gemini, npm, Git/local, and
  archive forms only to the extent the refreshed native contract supports them.
  Qwen's converter owns that conversion; skilltap records it as native and does
  not relabel the result as a portable plugin.
- Build the concrete source component graph before claiming conversion faithful.
  Skills, MCP, agents, commands, and context files are assessed independently;
  successful process exit alone cannot establish hook/agent/command semantic
  equivalence. Required unsupported conversion blocks; optional loss stays
  acknowledgment-gated.
- `SkillProjectionPort` returns `~/.qwen/skills` and
  `<project>/.qwen/skills`; project links come from the shared contract.
  Extension-owned skills remain native plugin children and are not duplicated
  into standalone roots.
- Managed MCP merges only `mcpServers` in `~/.qwen/settings.json` or
  `<project>/.qwen/settings.json`, preserving every unknown sibling member and
  unrelated server. Stdio/HTTP/SSE are accepted only when Qwen's exact field,
  auth-reference, and transport semantics are faithful.
- A write is not effectively complete until a new bounded Qwen process in the
  exact scope observes the server. The adapter reports `restart_required` or
  session-load failure as attention, never as filesystem drift.
- Native extension source, converted identity, enabled state, installed
  revision, and managed projection manifest remain distinct target-local
  evidence. Removal follows the applied representation and never deletes the
  other representation.

**Acceptance criteria**:

- [ ] `qwen` is a managed distribution target in the registry with one exact
      source-validated profile and observe-only unknown versions.
- [ ] Native source and extension lifecycle preserve Qwen scope, conversion
      identity, enablement, updates, and post-observed state without cache writes.
- [ ] Conversion compatibility is component-evidenced; unsupported required
      behavior blocks and optional loss requires `--yes`.
- [ ] Complete standalone skills pass both scopes and shared project linking.
- [ ] Managed `settings.json` projection preserves unknown fields and supports
      only faithful stdio/HTTP/SSE definitions.
- [ ] Effective verification uses a fresh scoped session and immediate repeat
      produces no source, settings, artifact, or state changes.

---

### Unit 4: GitHub Copilot CLI adapter

**Files**:

- `crates/harnesses/src/adapters/copilot.rs` (new) — detection/profile, native
  plugin lifecycle, source assessment, scoped paths, policy/trust observation.
- `crates/harnesses/src/adapters/copilot_managed.rs` (new) — managed skill/MCP
  projection and effective JSON observation.
- `crates/harnesses/src/adapters/mod.rs`, `crates/harnesses/src/registry.rs`, and
  `crates/harnesses/src/lib.rs` — exports and canonical registry entry.
- `crates/harnesses/tests/detection.rs`,
  `crates/harnesses/tests/lifecycle_scope.rs`, and
  `crates/harnesses/tests/normalization.rs` — target contract tests.
- `crates/test-support/src/harness_profile.rs` —
  `FakeHarnessProfile::copilot()`.

**Story**: `epic-expanded-harness-support-native-coexistence-copilot`

```rust
pub struct CopilotAdapter;
pub struct CopilotLifecycle;
pub struct CopilotSkillProjection;
pub struct CopilotNativeDistribution;
pub struct CopilotManagedProjection;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopilotEffectiveMcpObservation {
    pub declared: BTreeMap<NativeId, Fingerprint>,
    pub effective: BTreeMap<NativeId, Fingerprint>,
    pub policy: CopilotPolicyHealth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopilotPolicyHealth {
    Allowed,
    TrustRequired,
    EnterpriseBlocked,
    Unknown,
}
```

**Implementation notes**:

- Refresh the official Copilot CLI reference and validate a clean binary before
  adding exact profile constants. Pin plugin/marketplace argv and scope
  behavior, `--version`, structured plugin list, and `mcp list|get --json`
  schemas. Repository operations use the bounded project working directory.
- Native distribution assessment recognizes Copilot `plugin.json` marketplace
  forms and explicitly documented Claude marketplace forms. Installed plugin
  directories and marketplace caches are observed read-only; declarative
  `enabledPlugins` and imperative lifecycle are normalized as native declared
  state, not competing desired resources.
- `SkillProjectionPort` chooses canonical `~/.agents/skills` globally and
  `<project>/.agents/skills` for project scope. The project contract therefore
  returns `NotRequired`; Copilot's alternate `.github/skills` and
  `.claude/skills` roots are observation/preference evidence, never duplicate
  managed destinations.
- Managed user MCP targets `~/.copilot/mcp-config.json`; project MCP chooses
  canonical `<project>/.mcp.json`. Existing `.github/mcp.json`, plugin MCP, and
  user configuration remain independently observed. Same-name precedence is
  compared against `mcp list|get --json`; skilltap does not merge two project
  files or rewrite a higher-precedence unmanaged declaration.
- The JSON codec preserves unknown document members, unrelated servers, and
  environment/header references without persisting secret values. Removal
  requires exact target-local ownership of the server entry.
- Repository trust and enterprise allowlists narrow effective state only. A
  correctly written declaration blocked by policy produces a stable actionable
  health finding, not a drift rewrite and not a capability grant.
- Copilot plugin-owned skills/MCP remain native children. Managed fallback
  projects only components whose source has no faithful native distribution;
  native and managed fingerprints/identities never coalesce by name.

**Acceptance criteria**:

- [ ] `copilot` is registry-derived, outside first-party skilltap bootstrap, and
      has an exact validated profile with observe-only unknown versions.
- [ ] Native marketplace/plugin lifecycle preserves source, qualified identity,
      scope, enablement, revision, and cache ownership.
- [ ] Project standalone skills consume canonical `.agents/skills` with no
      redundant link or copy; complete global skills remain intact.
- [ ] Managed user/project MCP preserves unknown fields and alternate
      declarations, and structured list/get observation distinguishes declared
      from effective state.
- [ ] Trust/enterprise blocks are attention findings and never treated as
      filesystem drift or silently bypassed with managed duplication.
- [ ] Native plugin removal and managed component removal affect only the
      representation proven owned by the target binding; immediate repeat is a
      no-op.

---

### Unit 5: Integrated coexistence acceptance matrix

**Files**:

- `crates/test-support/src/harness_profile.rs`,
  `crates/test-support/src/managed_acceptance.rs`, and
  `crates/test-support/src/integration.rs` — fixture profiles and isolated roots
  for all three adapters.
- `crates/harnesses/tests/detection.rs`,
  `crates/harnesses/tests/lifecycle_scope.rs`,
  `crates/harnesses/tests/normalization.rs`, and a focused
  `crates/harnesses/tests/native_coexistence.rs` (new) — adapter/native
  contracts.
- `crates/cli/src/application/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` — registry/config/help, route,
  lifecycle, status/JSON, partial failure, and idempotency acceptance.

**Story**: `epic-expanded-harness-support-native-coexistence-acceptance`

**Implementation notes**:

- Extend the existing fixture profile constructors; do not encode target paths
  in `snapshot_native_roots` branches. Profiles declare isolated native roots,
  skill roots, MCP documents, lifecycle dialect, reload probe, and optional
  managed projection contract.
- Run both the generic adapter acceptance matrix and managed projection matrix
  for each target. Add coexistence scenarios above those matrices rather than
  weakening their required evidence.
- Native tests invoke fake or isolated real binaries only through bounded
  executable resolution/direct argv. No test reads the operator's HOME, XDG,
  repository, plugin cache, trust settings, or real credentials.
- Every mutating scenario repeats immediately and expects no operation. Capture
  no-follow snapshots before/after conflicts and removals.

**Acceptance criteria**:

- [ ] Registry/help/config/`--target all` include `droid`, `qwen`, and `copilot`
      from `TargetRegistry::canonical()` and keep first-party bootstrap limited
      to Codex/Claude.
- [ ] Each exact validated binary/profile passes detection, both scopes,
      structured observation, native lifecycle, complete skill, MCP, reload,
      drift, removal, and immediate-repeat contracts; nearby/unknown versions
      are observe-only.
- [ ] One plugin native on Droid, managed on Qwen, and native on Copilot retains
      three independent target bindings, identities, revisions, ownership
      classes, and journals through update and target-local removal.
- [ ] Native and managed resources with equal names coexist without
      fingerprint/name coalescing or cache mutation.
- [ ] Project skill paths prove Droid/Qwen relative links and Copilot canonical
      no-op while preserving complete siblings and unmanaged native-only skills.
- [ ] Factory precedence, Qwen restart, and Copilot trust/enterprise failures
      are distinct actionable health findings with plain/JSON parity.
- [ ] Partial native failure stops only dependent operations, re-observes exact
      target state, preserves successful siblings, and yields the documented
      recovery plan.
- [ ] `cargo test --workspace --all-targets`, Clippy with warnings denied,
      formatting, and `git diff --check` pass before feature review.

## Implementation Order

1. `epic-expanded-harness-support-native-coexistence-contract` — Unit 1,
   `depends_on: []`.
2. `epic-expanded-harness-support-native-coexistence-factory` — Unit 2,
   `depends_on: [epic-expanded-harness-support-native-coexistence-contract]`.
3. `epic-expanded-harness-support-native-coexistence-qwen` — Unit 3,
   `depends_on: [epic-expanded-harness-support-native-coexistence-contract]`.
4. `epic-expanded-harness-support-native-coexistence-copilot` — Unit 4,
   `depends_on: [epic-expanded-harness-support-native-coexistence-contract]`.
5. `epic-expanded-harness-support-native-coexistence-acceptance` — Unit 5,
   `depends_on: [epic-expanded-harness-support-native-coexistence-factory,
   epic-expanded-harness-support-native-coexistence-qwen,
   epic-expanded-harness-support-native-coexistence-copilot]`.

The three adapter checkpoints are graph-independent after the shared contract,
but the normal delivery remains one feature owner carrying them sequentially
(Factory, Qwen, Copilot) to reuse the same source/profile/fixture context. They
are not three default implementation workers. The final acceptance checkpoint
integrates all routes and target-local coexistence behavior.

`work-view --blocking` was run for every story receiving a sibling dependency
before the edges were written; no existing dependents were found, so the graph
introduces no cycle.

## Simplification

- Replace `managed_project_lifecycle()`'s managed-first boolean with one
  evidence-based route. Do not add three CLI target branches.
- Generalize `ManagedProjectionContext::project` and
  `plan_managed_project_lifecycle` to concrete scope once; do not clone the
  managed lifecycle for global paths or each adapter.
- Reuse `MaterializationPlan` for native-versus-managed component evidence;
  do not invent a fidelity score or universal manifest.
- Reuse the project-skill link service verbatim. Adapters supply roots and
  compatibility only.
- Share a bounded JSON object-member merge helper for the three `mcpServers`
  documents while keeping each adapter's destination, precedence, transport,
  reload, and effective observation logic private.
- Retain native caches as observation-only evidence. Remove no cache helper and
  introduce no cache writer.
- Keep existing target-local state rather than adding a representation field,
  route table, or second inventory.
- Extend the two acceptance matrices and fixture profiles instead of creating a
  third adapter framework.

No separate cleanup/refactor story is warranted: each simplification is coupled
to the contract or adapter checkpoint that proves the replacement behavior.

## Testing

- **Pure route tests:** prior binding pinning, marketplace inheritance,
  faithful-native preference, managed strict-superset selection, equal partial
  native preference, incomparable partial block, required-component block, and
  contradictory state. Protects the central coexistence decision.
- **Profile/argv contract tests:** exact known and neighboring unknown versions,
  direct argument vectors, working directory/scope, bounded output, structured
  list schemas, and narrowing-only probes. Protects mutation authority.
- **Source/conversion tests:** target-native, documented converted, managed-only,
  malformed, pinned/unpinnable, optional-loss, and required-loss sources.
  Protects faithfulness without a universal plugin format.
- **Project skill tests:** complete canonical directory, Droid/Qwen relative
  targets, Copilot no-op, conflicts, repair, targeted removal, and repeat.
  Protects consumption of the prerequisite contract.
- **MCP codec tests:** global/project paths, unknown field preservation,
  transport/auth references, same-name precedence, drift, owned removal, and
  empty-document ownership. Protects target-native config boundaries.
- **Effective observation tests:** Factory auto-reload, Qwen fresh-session load,
  Copilot JSON list/get plus trust/policy distinction. Protects declared versus
  effective state.
- **Compiled acceptance:** multi-target mixed representation, partial native
  failure/recovery, plain/JSON parity, target-local updates/removals, and
  immediate-repeat idempotency. Protects the product promise.
- **Test economy:** no tests for trivial getters, static labels, or every native
  parser error string. Exact profile authority, routing, ownership, precedence,
  codecs, and user-visible outcomes earn coverage.

## Risks

- **Riskiest assumption — current native command contracts:** research attests
  the lifecycle families but not exact versions and every argv/JSON shape. The
  implementation must refresh official references and validate clean binaries;
  no guessed profile gains authority. If a target cannot produce stable bounded
  observation, register it observe-only and keep its story incomplete rather
  than overclaim support.
- **Native conversion fidelity:** Factory/Qwen/Copilot accept overlapping source
  formats, but acceptance does not prove every component equivalent. Concrete
  source graph comparison and fresh effective observation are required. The
  fallback is a visible managed/partial/blocked plan, not relabeling conversion
  as faithful.
- **Scope-general managed projection:** generalizing the Codex project contract
  could regress its mature path. Codex byte/operation/state regression tests run
  in Unit 1 before new adapters consume global scope; existing native global
  routing stays pinned.
- **Precedence ambiguity:** user/project/plugin definitions can coexist with
  different precedence. Adapters compare declared and effective layers and
  block on unproven collisions. They never resolve ambiguity by deleting an
  unmanaged higher-precedence entry.
- **Representation migration:** a later native distribution may become
  available for an existing managed binding. Automatic migration would mix
  ownership and removal semantics, so this feature deliberately pins the old
  representation. A future explicit migration workflow can plan both removal
  and installation with rollback evidence.
- **Qwen restart and Copilot policy:** filesystem correctness may not imply
  effective load. Fresh-session and structured policy probes can fail in CI or
  enterprise environments; fixtures keep ordinary tests hermetic, while exact
  real-binary validation is an explicit profile admission gate.
- **Shared JSON helper scope:** all three documents contain `mcpServers`, but
  destination and semantics differ. Share only bounded member-preserving merge
  mechanics; if one target's schema diverges, its adapter keeps a separate codec
  behind the same managed port.

## Other agent review

- **Effective review weight:** standard (caller/default).
- **Design-time advisory decision:** the cross-cutting route and ownership
  contract would normally warrant one independent standard pass, but the caller
  explicitly prohibited nested agents and peer mechanisms. Per workflow policy,
  design review is non-blocking; the design proceeded with high-effort direct
  grounding and recorded the degradation.
- **Implementation closure:** one independent standard feature review remains
  required after all child checkpoints verify. Child stories advance directly
  to done and never become review units.
