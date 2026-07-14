---
id: epic-expanded-harness-support-pi
kind: feature
stage: implementing
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links, epic-expanded-harness-support-pi-hook-research]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-14
---

# Pi Compound Target Adapter

## Brief

Deliver Pi as a conditional compound target whose mutable profile requires the
Pi runtime plus separately observed, compatible user-installed MCP and Claude
Code hook-compatibility extensions. Their identities, versions, capabilities,
health, and ownership remain distinct; skilltap neither attributes extension
behavior to Pi core nor silently adopts existing companion packages.

The adapter consumes the shared whole-skill and MCP lifecycle only when an exact
compiled compound profile and fresh health evidence authorize the requested
mutation. Missing, incompatible, unknown, inert, or unverified companions keep
Pi observe-only with actionable health output. The feature covers global and
project scope, project-skill projection metadata, update and drift behavior,
isolated validation, and the common adapter acceptance evidence.

## Epic context

- Parent epic: `epic-expanded-harness-support`.
- Prerequisites complete: typed target registry, target-agnostic managed
  projection, canonical project skills with registry-derived relative links,
  and full Pi hook research.
- Execution posture: one cohesive Sol xhigh feature owner. Child stories are
  dependency checkpoints, not parallel worker assignments.
- Effective review weight: standard. Child stories verify directly to `done`;
  the feature receives one independent review pass after integrated
  verification, followed by receiver adjudication and fixes without a second
  pass.

## Research decision incorporated

The completed engagement materially narrows the original brief:

- Pi core `0.80.6` is the exact attested runtime.
- `pi-mcp-adapter@2.11.0` is a distinct user-installed package with documented
  global/project MCP files, but no attested non-interactive initialization
  health surface.
- `@hsingjui/pi-hooks@0.0.2` is the distinct current Claude-hook companion. It
  is a best-effort command-hook subset, not a faithful Claude Code hook
  implementation: event coverage, handler types, async behavior, Stop timing,
  payload names, matcher behavior, timeout, and blocking semantics materially
  differ.
- Package presence is not activation. The observed hook package can be listed
  and loaded while the `hooks` settings key is absent and every callback is
  inert.
- Pi package entries, checkout manifests, hook settings, and MCP files remain
  user/harness-owned. Observation does not adopt them or grant skilltap removal
  or update authority.

Therefore the current exact tuple is a **verified observe-only compound
profile**: it may provide precise status and compatibility evidence, but every
mutation capability is `Unsupported`. This feature must not add unreachable Pi
MCP codecs or package lifecycle code merely to appear complete. A later
source-attested companion can add a mutation-capable tuple through the shared
ports only after it clears semantic, activation, trust, ownership, and
revalidation gates.

## Grounding summary

The completed foundations provide the correct seams:

- `TargetRegistry` and `HarnessAdapter` in
  `crates/harnesses/src/registry.rs` are the only target registry and adapter
  dispatch boundary. `HarnessId` remains generic; Pi adds one adapter and one
  canonical registry entry.
- `CapabilityProfileSelection` and `ScopedCapabilitySets` in
  `crates/core/src/domain/installation.rs` already distinguish exact compiled
  profiles from unknown versions and enforce narrowing-only runtime evidence.
  The missing concept is a typed set of required companion observations from
  which a compound profile is selected.
- `HarnessObservation` and registered findings in
  `crates/core/src/domain/observation.rs` and
  `crates/core/src/domain/resource/finding.rs` are safe normalized evidence.
  Companion packages must not enter `HarnessObservation.resources`, because
  that collection feeds adoption; they need a separate non-adoptable profile
  component contract.
- Native status currently calls `adapter.select_profile(native_version)` and
  `adapter.observe(...)` independently in
  `crates/cli/src/application/status.rs`. Mutation paths call
  `configured_native_profile` for native lifecycle, while standalone skill
  publication uses `SkillProjectionPort` without a native profile check. Pi
  therefore needs one conditional-profile resolver consumed by status and all
  Pi-target mutation entry points; omitting the standalone-skill guard would
  bypass the compound contract.
- `ManagedProjectionPort::plan` already owns target-native tree/file planning,
  complete projection evidence, ownership, drift, rollback, pending recovery,
  and immediate-repeat behavior. Pi does not implement this port while every
  compiled Pi tuple blocks mutation. The first future authorized tuple must use
  this port rather than creating Pi-local lifecycle machinery.
- Project standalone skills are already canonical at
  `<project>/.agents/skills/<name>`. Pi natively consumes that root, so
  `PiSkillProjection` produces the existing canonical/no-link path. The
  adapter also observes `.pi/skills` as an unmanaged precedence surface; it
  never copies or links the same skill there.
- The root-confined filesystem port supplies bounded no-follow reads for
  `settings.json`, package manifests, and MCP configuration. Native CLI
  execution remains direct-argv and bounded. This design deliberately avoids
  parsing `pi list` human output because settings plus documented package roots
  and manifests provide stronger structured evidence and `pi list` reports no
  version or health.

The codebase map was produced by direct reads only. The caller prohibited nested
agents and peer mechanisms, so no design-time advisory dispatch ran; standard
implementation review remains required at feature closure.

## Design decisions

- **Aggregate profile is not Pi core.** `PiAdapter::select_profile` identifies
  the core runtime only. A new optional `ConditionalProfilePort` observes the
  MCP and hook companions, selects an exact compiled tuple, and narrows it with
  activation/trust evidence. Status renders the core and each companion as
  separate rows and labels the effective row `compound_profile`.
- **Runtime evidence can only narrow.** The port has separate
  `inspect_components` and `select_compiled_profile` methods. Core constructs
  the effective profile by calling `compiled.narrow(report.narrowing())`; an
  observed package or config file can never turn an unverified/unsupported
  capability into `Supported`.
- **Known does not mean mutable.** Exact tuple
  `pi 0.80.6 + pi-mcp-adapter 2.11.0 + @hsingjui/pi-hooks 0.0.2` receives a
  stable compiled profile id so status can distinguish known incompatibility
  from unknown versions. Its observation capabilities are precise, but
  `skill.*`, `plugin.*`, and `marketplace.*` mutations are explicitly
  `Unsupported` because the required hook companion is semantically partial
  and MCP initialization remains unverified.
- **Missing/unknown tuples fail closed.** A missing package, mismatched manifest,
  unknown package version, malformed settings, unverified project trust, or
  unsupported settings shape yields an unknown or narrowed observe-only
  profile. It never falls back to core-only mutation.
- **Companion rows are not resources.** `ProfileComponentObservation` is an
  ephemeral normalized type outside the desired/observed resource graph.
  `adopt` cannot ingest it. Existing packages remain `Ownership::Harness` and
  render `adoptable=false`.
- **Settings are declaration evidence, manifests are version evidence.** Global
  `~/.pi/agent/settings.json` and project `.pi/settings.json` determine package
  declaration and precedence. The exact package checkout's `package.json`
  determines installed name/version/entrypoint. Neither fact alone is called
  healthy.
- **No human-output package parser.** `pi list` is settings-derived, has no JSON
  mode in the attested contract, and omits version/health. The adapter reads
  documented JSON settings and bounded manifests instead of adding a brittle
  parser that yields weaker evidence.
- **MCP config is declared, not proven effective.** The adapter observes
  `~/.config/mcp/mcp.json`, `~/.pi/agent/mcp.json`, project `.mcp.json`, and
  project `.pi/mcp.json` with documented precedence. It never writes or treats
  `mcp-cache.json` as configuration. Without a supported non-interactive health
  response, activation remains `Unverified`.
- **Hook configuration is not compatibility.** A missing `hooks` key is
  `Inert`; a present valid key is `ConfiguredUnverified`. In both cases version
  `0.0.2` remains `Partial`, so configuration cannot upgrade the semantic
  result.
- **No implicit package ownership.** The adapter exposes neither
  `NativeLifecycleVector` nor `ManagedProjectionPort` for current Pi. It never
  invokes `pi install/remove/update`, edits the packages array, or claims
  package drift. Changes to user-owned package versions/configuration are fresh
  health evidence, not managed-resource drift.
- **All Pi-target writes share one guard.** A conditional-profile capability
  check runs before standalone skill install/update/remove, plan/sync
  reconciliation, daemon apply, and any future managed projection dispatch.
  `harness enable pi` and read-only adoption remain allowed because they change
  skilltap policy/inventory, not Pi native state.
- **No current apply-time companion revalidation abstraction.** No current Pi
  profile can reach apply, so adding an unused evidence-binding execution port
  would be speculative. The first profile that changes a Pi mutation
  capability to `Supported` must also bind companion evidence to operation ids
  and re-observe it under the configuration lock before apply. This is a hard
  acceptance gate, not optional future cleanup.
- **Project skills use the completed contract.** Pi reports
  `<project>/.agents/skills` as its destination; the shared project-skill
  planner returns `Canonical` and creates no duplicate `.pi/skills` tree or
  relative link. Mutation still blocks for Pi until the compound profile allows
  `skill.install`/`skill.update`/`skill.remove`.
- **No UI work.** This is a non-interactive CLI/domain/adapter feature; there is
  no screen or flow surface.

## Architectural choice

**Chosen — optional conditional-profile port with a known observe-only Pi
adapter.** Core owns normalized profile-component and narrowing contracts;
`skilltap-harnesses` owns the optional port and Pi-specific settings/manifest
codecs; CLI composition resolves the effective profile once per target/scope
and uses it for status plus every Pi mutation guard. The current adapter
registers Pi and its skill observation root but advertises no lifecycle port.

**Rejected — flatten companions into Pi core capabilities.** Marking MCP/hooks
on `PiAdapter::select_profile(0.80.6)` would falsely claim Pi core supplies them
and would lose package identity, version, activation, ownership, and update
independence.

**Rejected — register Pi only when companions are installed.** The canonical
registry is build-time truth, not machine state. Conditional presence belongs
in status/profile evidence; otherwise `harness enable pi`, diagnostics, and
missing-companion next actions would disappear exactly when needed.

**Rejected — model companions as ordinary observed plugins.** That would feed
user-owned packages into adoption and risk implicit inventory/ownership. A
profile component is a required capability provider, not a managed plugin
resource.

**Rejected — implement a dormant Pi managed-projection codec.** No current
profile can authorize it, so the code would be unreachable and unverified.
When a companion clears the gate, Pi will implement the already-completed
`ManagedProjectionPort` using its attested MCP files and shared ownership
machinery.

## Trickiest unit first

The conditional profile and cross-workflow mutation guard are the highest-risk
unit. Observation is not enough: the design fails if a healthy-looking package
file can widen authority, if companions leak into adoption, or if standalone
skill publication bypasses the native lifecycle profile. Unit 1 therefore
establishes narrowing-only composition and non-adoptable component evidence;
Unit 3 applies that result before any Pi-target filesystem or native operation.
The concrete adapter hangs between those two contracts rather than owning a
private authorization path.

## Implementation Units

### Unit 1: Conditional compound-profile contracts

**Files**:

- `crates/core/src/domain/conditional_profile.rs` (new) — normalized component
  identity, presence, activation, compatibility, report, and safe profile
  composition.
- `crates/core/src/domain/mod.rs` — exports.
- `crates/core/src/domain/resource/finding.rs` — registered companion/profile
  findings and bounded component field.
- `crates/harnesses/src/conditional_profile.rs` (new) — optional adapter port
  and inspection context.
- `crates/harnesses/src/registry.rs` and `crates/harnesses/src/lib.rs` — default
  optional accessor and exports.

**Story**: `epic-expanded-harness-support-pi-profile`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileComponentRole {
    McpCompanion,
    HookCompanion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileComponentPresence {
    Missing,
    Present,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileComponentActivation {
    Inert,
    ConfiguredUnverified,
    Effective,
    TrustRequired,
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileComponentCompatibility {
    Compatible,
    Partial,
    Incompatible,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponentObservation {
    pub id: NativeId,
    pub package: NativeId,
    pub role: ProfileComponentRole,
    pub required: bool,
    pub declared_scope: Option<CapabilityScope>,
    pub presence: ProfileComponentPresence,
    pub version: Option<NativeVersion>,
    pub activation: ProfileComponentActivation,
    pub compatibility: ProfileComponentCompatibility,
    pub ownership: Ownership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponentSet(BTreeMap<NativeId, ProfileComponentObservation>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalComponentReport {
    components: ProfileComponentSet,
    narrowing: ScopedCapabilitySets,
    findings: Vec<ObservationFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalProfileObservation {
    profile: CapabilityProfileSelection,
    components: ProfileComponentSet,
    findings: Vec<ObservationFinding>,
}

impl ConditionalProfileObservation {
    pub fn compose(
        compiled: CapabilityProfileSelection,
        report: ConditionalComponentReport,
    ) -> Result<Self, ConditionalProfileError>;
    pub const fn profile(&self) -> &CapabilityProfileSelection;
    pub const fn components(&self) -> &ProfileComponentSet;
    pub fn findings(&self) -> &[ObservationFinding];
    pub fn mutation_support(
        &self,
        scope: &Scope,
        capability: &CapabilityId,
    ) -> CapabilitySupport;
}

pub struct ConditionalProfileContext<'a> {
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
    pub maximum_manifest_bytes: u64,
}

pub trait ConditionalProfilePort: Sync {
    fn inspect_components(
        &self,
        context: &ConditionalProfileContext<'_>,
    ) -> Result<ConditionalComponentReport, ConditionalProfileError>;

    fn select_compiled_profile(
        &self,
        runtime_version: &NativeVersion,
        components: &ProfileComponentSet,
    ) -> CapabilityProfileSelection;
}

pub trait HarnessAdapter: Sync {
    // existing methods
    fn conditional_profile(&self) -> Option<&dyn ConditionalProfilePort> {
        None
    }
}
```

**Implementation notes**:

- `ConditionalProfileObservation::compose` is the sole composition path and
  calls `compiled.narrow(report.narrowing())`. A port cannot widen a compiled
  capability with runtime presence/configuration evidence.
- `ProfileComponentSet` rejects duplicate ids. Findings must match the requested
  harness/scope and use only registered authored summaries plus typed scalar
  fields; no settings bytes, paths, argv, package JSON, or parser messages may
  enter output.
- Add finding codes/summaries for required companion missing, version
  unverified, inactive/unverified, semantically incompatible/partial, and
  compound mutation unavailable. Add one bounded `ProfileComponent(NativeId)`
  field; do not add arbitrary string context.
- This contract is ephemeral and has no storage wire. Package presence and
  health are re-observed for every status/authorization request and never enter
  `state.json` as Pi-core evidence.

**Acceptance criteria**:

- [ ] Component ids are unique and MCP/hook roles remain distinct in typed
      output.
- [ ] Runtime narrowing can preserve or reduce compiled support but cannot
      upgrade `Unverified`/`Unsupported` to `Supported`.
- [ ] An unknown compiled tuple never exposes mutation authority even when all
      runtime booleans look healthy.
- [ ] Profile components cannot enter desired/observed resource graphs or
      adoption candidates.
- [ ] Existing Codex/Claude adapters compile unchanged because the optional port
      defaults to absent.

---

### Unit 2: Pi core adapter and separate companion observation

**Files**:

- `crates/core/src/runtime/error.rs` and
  `crates/core/src/runtime/paths.rs` — validated Pi home/package roots and
  optional `PI_PACKAGE_DIR` boundary.
- `crates/harnesses/src/adapters/pi.rs` (new) — runtime detection, core
  observation paths, skill projection, target identity.
- `crates/harnesses/src/adapters/pi_profile.rs` (new) — companion settings,
  manifest, configuration, compiled tuple, and health narrowing.
- `crates/harnesses/src/adapters/pi_settings.rs` (new, private) — bounded strict
  JSON extraction for supported Pi settings/package forms.
- `crates/harnesses/src/adapters/mod.rs` and `crates/harnesses/src/lib.rs` —
  module/export wiring without canonical registration yet.

**Story**: `epic-expanded-harness-support-pi-adapter`

```rust
pub struct PiAdapter;
pub struct PiSkillProjection;
pub struct PiConditionalProfile;

impl HarnessAdapter for PiAdapter {
    fn identity(&self) -> TargetIdentity;
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort>;
    fn conditional_profile(&self) -> Option<&dyn ConditionalProfilePort>;
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath>;
}

impl SkillProjectionPort for PiSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath>;
}

impl ConditionalProfilePort for PiConditionalProfile {
    fn inspect_components(
        &self,
        context: &ConditionalProfileContext<'_>,
    ) -> Result<ConditionalComponentReport, ConditionalProfileError>;

    fn select_compiled_profile(
        &self,
        runtime_version: &NativeVersion,
        components: &ProfileComponentSet,
    ) -> CapabilityProfileSelection;
}
```

**Implementation notes**:

- Identity is `pi` / `Pi`, managed distribution; the configured executable is
  `pi`. `pi --version` must decode one bounded token. Exact `0.80.6` is the
  attested core profile; every other version remains unknown/observe-only.
- `PlatformPaths::pi_home()` is `~/.pi/agent`. Resolve the package root from
  documented `PI_PACKAGE_DIR` when set, otherwise from the scoped Pi npm
  package roots. All manifest/settings reads use descriptor-relative no-follow
  bounded reads; never join an untrusted native-output path and call ordinary
  `std::fs`.
- Observe global `~/.agents/skills`, `~/.pi/agent/skills`, settings, and the four
  MCP declaration surfaces. At project scope observe `<project>/.agents/skills`,
  `<project>/.pi/skills`, `.pi/settings.json`, `.mcp.json`, and `.pi/mcp.json`.
  `surface_labels` remain static adapter-authored ids, never raw paths.
- `PiSkillProjection` returns `~/.agents/skills` globally and
  `<project>/.agents/skills` for projects. It uses the conservative portable
  compatibility result. Pi-specific skill roots are observed native siblings,
  not additional publication destinations.
- Resolve exact package declaration precedence from the supported Pi settings
  shapes. Project package identity overrides global identity; hook arrays retain
  their independently attested global-then-project concatenation semantics.
  Unknown settings shapes produce `NativeShapeUnsupported`/unverified health,
  never absence or compatibility.
- Read each declared npm package manifest and require the exact expected package
  name, version token, and Pi extension entrypoint. A package directory without
  a matching settings declaration is installed-but-inert evidence, not an active
  companion.
- MCP package identity is `pi-mcp-adapter`; exact attested version is `2.11.0`.
  Mark static mapping compatibility `Compatible`, but activation
  `ConfiguredUnverified`/`Unverified` because current evidence cannot prove
  extension initialization non-interactively.
- Hook identity is `@hsingjui/pi-hooks`; exact attested version is `0.0.2` and
  expected entrypoint is `./src/pi-hooks.ts`. Static compatibility is always
  `Partial` for this version. Missing `hooks` is `Inert`; a valid nonempty hooks
  object is `ConfiguredUnverified`. Neither becomes `Effective`.
- Project package/config activation narrows to `TrustRequired` when trust cannot
  be positively observed. The adapter never edits trust or infers it from file
  presence.
- The exact compiled tuple id is
  `pi-0-80-6-mcp-2-11-0-hooks-0-0-2`. Both scope capability sets mark
  `harness.observe` and `skill.observe` supported, MCP effectiveness unverified,
  hook compatibility unsupported, and every mutation (`skill.*`, `plugin.*`,
  `marketplace.*`) unsupported. Unknown/missing component versions return an
  unknown profile with mutation unavailable.
- `PiAdapter` intentionally returns `None` for `native_lifecycle()` and
  `managed_projection()` and retains the default false managed-lifecycle gate.
  It does not call Pi package commands or write package/settings/MCP files.

**Acceptance criteria**:

- [ ] Core, MCP companion, and hook companion have independent ids, versions,
      activation, compatibility, scope, ownership, and findings.
- [ ] Exact current tuple is recognized but exposes no mutation capability.
- [ ] Missing/mismatched/unknown manifests, missing package declarations,
      malformed settings, and unverified trust fail closed without hiding the
      healthy sibling component.
- [ ] A configured `0.0.2` hook remains partial; package/config presence cannot
      upgrade it.
- [ ] MCP files are observed in documented precedence order, while cache and
      secret values never enter output or state.
- [ ] Project skill destination equals canonical `.agents/skills`, producing no
      adapter-authored link/copy behavior.

---

### Unit 3: Registry, status, mutation guard, and non-adoption integration

**Files**:

- `crates/harnesses/src/registry.rs` — add Pi to the canonical registry order
  and contract tests.
- `crates/cli/src/application/conditional_profile.rs` (new) — one resolver and
  capability guard shared by status and mutating commands.
- `crates/cli/src/application.rs` — module/composition wiring.
- `crates/cli/src/application/status.rs` — effective compound profile,
  companion rows/findings, and actionable output.
- `crates/cli/src/application/lifecycle.rs` and
  `crates/cli/src/application/project_skills.rs` — guard standalone skill and
  reconciliation writes for targets with a conditional profile.
- `crates/cli/src/application/reconciliation.rs` — blocked Pi operations in
  plan/sync without native writes.
- `crates/cli/src/outcome.rs` only if a typed static field is required; do not
  add a Pi-specific renderer path.

**Story**: `epic-expanded-harness-support-pi-integration`

```rust
pub(super) struct ResolvedConditionalProfile {
    pub core_version: NativeVersion,
    pub observation: ConditionalProfileObservation,
}

fn resolve_conditional_profile(
    registry: &TargetRegistry,
    config: &ConfigDocument,
    target: &HarnessId,
    scope: &Scope,
    paths: &PlatformPaths,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    filesystem: &dyn ConfinedFileSystem,
) -> Result<Option<ResolvedConditionalProfile>, ConditionalProfileResolutionError>;

fn require_target_mutation_capability(
    resolved: Option<&ResolvedConditionalProfile>,
    capability: &CapabilityId,
    scope: &Scope,
) -> Result<(), ErrorDetail>;
```

**Implementation notes**:

- `resolve_conditional_profile` detects and version-binds the configured
  executable through the existing bounded native process path, calls the Pi
  component observer, selects the compiled tuple, and composes narrowing. It is
  the only CLI entry point for conditional profile resolution; status and
  mutation must not duplicate package/health logic.
- Adapters without a conditional port return `None` and preserve existing
  Codex/Claude behavior. Pi resolution errors produce attention-required typed
  output rather than falling through to ordinary skill publication.
- Add Pi once to `TargetRegistry::canonical()`. Help, enable/disable, config,
  harness list, target validation, and `--target all` derive from the registry.
  `DistributionSurface::Managed` keeps first-party bootstrap Codex/Claude-only.
- Status renders one core row plus exactly two required companion rows with
  static keys: role, package identity, presence, declared scope, version,
  activation, compatibility, ownership, required, and `adoptable=false`. It
  separately renders `compound_profile`, profile id/authority, and
  `mutation_authorized=false`. Plain and JSON derive from the same typed
  observation.
- Companion findings enter the harness observation finding list but companion
  rows do not enter `HarnessObservation.resources`. `adopt --from pi` may adopt
  ordinary observed Pi skills under existing rules; it cannot adopt the MCP or
  hook package from profile evidence.
- Guard all standalone skill install/update/remove paths before constructing a
  physical destination operation, including project canonical publication,
  global publication, reconciliation/sync repair, and daemon update. Use exact
  capability ids (`skill.install`, `skill.update`, `skill.remove`).
- Plugin/marketplace commands remain blocked because Pi exposes no lifecycle or
  managed projection port. If a future Pi port appears, dispatch must call the
  same guard for the corresponding exact capability before planning it.
- A canonical project skill physically present because another target owns it
  may be observed as loadable by Pi, but skilltap must not create Pi target
  state or claim a Pi apply result while the compound profile blocks mutation.
- Companion updates/removals remain user-owned native actions. Status reports
  unknown/incompatible versions and the research-trigger next action; daemon
  and foreground update never invoke `pi update`, rewrite package specs, remove
  a package, or overwrite hook/MCP config.
- Current hook incompatibility next action must not falsely say “install
  `@hsingjui/pi-hooks` to enable mutation.” It explains that the identifiable
  package is partial and no mutation-authorized hook companion is currently
  compiled. Missing MCP may still name `pi-mcp-adapter` as the required
  capability provider while stating that the aggregate remains observe-only.

**Acceptance criteria**:

- [ ] Registry/help/list/enable/status expose `pi`; bootstrap remains exactly
      Codex/Claude.
- [ ] Status always distinguishes Pi core, MCP companion, hook companion, and
      effective compound profile; no row attributes MCP/hooks to core.
- [ ] Companion rows and findings never become adoption candidates or persisted
      target resource state.
- [ ] Every current Pi skill/plugin/marketplace mutation path blocks before
      filesystem/native command execution, including plan/sync and daemon.
- [ ] A blocked Pi target does not prevent unrelated authorized targets in the
      same command from applying safely.
- [ ] Project `.agents/skills` is canonical/no-link metadata; `.pi/skills`
      conflicts are observed and preserved.
- [ ] User-owned package/config changes alter fresh status only and are never
      labeled skilltap drift.

---

### Unit 4: Isolated conditional-target acceptance evidence

**Files**:

- `crates/test-support/src/harness_profile.rs` — Pi version fixture and
  profile-carried Pi roots without target-id branching.
- `crates/test-support/src/conditional_profile.rs` (new) — isolated settings,
  package manifest, component-health, and immutable-snapshot fixtures.
- `crates/harnesses/tests/detection.rs` and Pi adapter unit/contract tests —
  exact version, paths, settings, manifests, and tuple selection.
- `crates/cli/src/application/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` — registry/status/adoption/mutation
  integration.

**Story**: `epic-expanded-harness-support-pi-acceptance`

**Implementation notes**:

- Extend `FakeHarnessProfile` through profile data, not `match id`, with exact
  `pi --version -> 0.80.6`, no lifecycle dialect, global/project Pi roots, and
  two companion manifest fixtures.
- Use `IsolatedMachine` HOME, XDG, package root, projects, settings, package
  manifests, skills, and MCP files. Never read or write the operator's real
  `~/.pi`, project `.pi/`, package directories, or binaries.
- Snapshot every native Pi/settings/package/MCP path before blocked mutations
  and assert byte-for-byte identity afterward. Also assert no Pi target binding,
  apply journal, ownership, or pending attempt was persisted.
- The general mutable adapter acceptance matrix is intentionally not claimed:
  current Pi cannot pass mutation acceptance. Add a conditional-target matrix
  whose success criteria are exact observation, safe blocking, target
  isolation, non-adoption, and repeated read-only determinism.

**Acceptance criteria**:

- [ ] Exact core with both exact packages, absent hooks, and no MCP config shows
      core reachable, MCP activation unverified, hooks inert/partial, and
      compound mutation unavailable.
- [ ] Adding valid hook config changes only activation to configured-unverified;
      compatibility remains partial and every mutation remains blocked.
- [ ] Missing MCP, missing hook, mismatched package name, unknown version,
      malformed manifest/settings, and project trust uncertainty each produce a
      distinct stable finding while preserving sibling observations.
- [ ] Global/project package precedence and hook concatenation are tested as
      separate rules; one is never substituted for the other.
- [ ] Pi uses canonical project `.agents/skills` with no redundant link. A
      pre-existing `.pi/skills` sibling is preserved and reported without
      adoption or overwrite.
- [ ] `adopt --from pi` excludes companion packages. `plan`, `sync`, standalone
      skill install/update/remove, plugin lifecycle, and daemon updates perform
      no Pi native mutation under every current fixture.
- [ ] Multi-target operations may apply an authorized sibling target while Pi
      reports attention required.
- [ ] Repeated status/profile resolution is deterministic and side-effect-free.
- [ ] Unknown Pi or companion versions remain observe-only; runtime file
      presence never grants a compiled profile.
- [ ] Plain/JSON output carry the same component ids, versions, activation,
      compatibility, ownership, profile result, warnings, next actions, and exit
      class.
- [ ] Full workspace tests, all-feature Clippy with warnings denied, formatting,
      and `git diff --check` pass before feature review.

## Implementation Order

1. `epic-expanded-harness-support-pi-profile` — Unit 1,
   `depends_on: []`.
2. `epic-expanded-harness-support-pi-adapter` — Unit 2,
   `depends_on: [epic-expanded-harness-support-pi-profile]`.
3. `epic-expanded-harness-support-pi-integration` — Unit 3,
   `depends_on: [epic-expanded-harness-support-pi-profile,
   epic-expanded-harness-support-pi-adapter]`.
4. `epic-expanded-harness-support-pi-acceptance` — Unit 4,
   `depends_on: [epic-expanded-harness-support-pi-integration]`.

`work-view --blocking <story-id>` was run for every story before adding sibling
dependencies; all returned no existing dependents, so the graph introduces no
cycle. The sequence is intentionally linear after the contract: current Pi is a
single conditional adapter, and parallelizing profile, status, and mutation
gates would create overlapping registry/composition write sets without reducing
risk.

## Simplification

- Reuse `CapabilityProfileSelection::narrow`; do not invent a second capability
  support algebra.
- Add one optional `ConditionalProfilePort`; do not special-case Pi throughout
  CLI target dispatch.
- Keep companion evidence outside the resource graph; do not add state schema,
  inventory entries, or adoption filters to undo an incorrect representation.
- Reuse `SkillProjectionPort` and the completed canonical project-skill planner;
  do not create Pi links or copied skill trees.
- Reuse root-confined reads and strict JSON; do not parse weaker `pi list`
  human output or inspect arbitrary package paths.
- Do not implement native package lifecycle, managed MCP projection, cache
  mutation, hook translation, instruction bridges, or update ownership while no
  compiled tuple can authorize them.
- Add one shared conditional mutation guard at composition boundaries; do not
  scatter Pi id checks or rely on absent ports as the only safety condition.
- Retain user-owned companion files untouched. There is no safe cleanup to fold
  into this observe-only feature.

No `[refactor]` or `[cleanup]` child is warranted. All new structure is required
to represent a genuinely compound target without false ownership or capability
claims.

## Testing

- **Profile contract tests:** duplicate component rejection, compiled-profile
  selection, narrowing-only health, unknown tuple authority, and non-adoptable
  separation. Protects the central safety invariant.
- **Adapter codec tests:** exact Pi version bytes, global/project paths, package
  precedence, supported settings shapes, manifest identity/version/entrypoint,
  MCP declaration precedence, hook activation, and static semantic
  classification. Protects native boundary truth without testing every parser
  branch.
- **Mutation regression tests:** every Pi-target write command blocks before a
  request enters a native/filesystem execution port, while unrelated targets
  continue. Protects the no-bypass requirement, especially standalone skills.
- **Compiled CLI tests:** registry-derived help/enable/list, component-separated
  status, JSON/plain parity, adoption exclusion, project canonical skill
  behavior, next actions, and exit code 2 for attention. Protects the
  agent-facing contract.
- **Immediate repeats:** status/profile resolution repeats with identical output
  and no files/state changed. There is no dishonest “mutation idempotency” claim
  for a target that is not allowed to mutate.
- **Test economy:** no getter tests, no snapshots of full help text, no raw
  parser-error assertions, and no mutable acceptance labels for Pi. Pin stable
  ids, profile/capability states, findings, paths, ownership, bytes, and output
  semantics.

## Risks

- **Riskiest assumption — an observe-only adapter still satisfies the current
  feature.** The original brief expected a mutable compound target, but the
  completed required research disproves the hook prerequisite. Shipping no Pi
  target would hide actionable state; shipping mutation would violate the
  foundation. A registered, deeply observed, explicitly blocked target is the
  only evidence-consistent result. The fallback is not a weaker hook promise;
  it is a future research refresh and new exact profile.
- **Standalone skill bypass:** current skill publication does not use native
  lifecycle profiles. Without Unit 3, Pi could mutate `.agents/skills` despite
  a blocked compound profile. The shared conditional guard and compiled tests
  cover global, project, sync, and daemon paths.
- **Verified profile wording:** the exact current tuple is “verified compiled”
  but has zero mutation authority. Output must say both facts and render
  `mutation_authorized=false`; it must not use “healthy” as shorthand for a
  recognized tuple.
- **MCP initialization uncertainty:** files and package manifest do not prove the
  extension initialized. The design preserves `Unverified` rather than reading
  cache as authority or invoking interactive `/mcp`. If a stable
  non-interactive health surface appears, it becomes runtime narrowing evidence,
  not automatic widening.
- **Project trust uncertainty:** no attested non-interactive trust state is safe
  to infer. Project activation remains trust-required/unverified. Missing
  positive trust evidence must not be rendered as definite user distrust.
- **Package layout drift:** package roots/settings forms can change. Exact core
  and companion versions gate decoding; malformed/unknown shapes fail closed.
  `PI_PACKAGE_DIR` is validated once in `PlatformPaths` and no raw package path
  from native output is trusted.
- **Future mutation revalidation:** current apply is unreachable, so this feature
  does not add speculative execution evidence. The first mutation-capable Pi
  profile must add under-lock re-observation of core executable identity,
  package declarations/manifests, versions, activation, and trust before any
  capability changes to `Supported`.
- **Sibling contract movement:** active `--all` work may generalize default
  binaries or effective-state probes first. Implementation should consume the
  realized registry contract and avoid parallel equivalents; the Pi-specific
  component/profile model remains distinct because ordinary effective MCP
  status does not model required package identity or hook semantics.

## Review posture

Design-time advisory review was risk-warranted but intentionally skipped because
the delegated endpoint forbids nested agents and peer mechanisms; design review
is non-blocking. Feature closure still requires the caller-selected standard
path: exactly one independent feature-level review, receiver adjudication,
material fixes, verification, and completion without a second independent pass.
