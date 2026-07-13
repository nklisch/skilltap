---
id: epic-expanded-harness-support-registry
kind: feature
stage: review
tags: []
parent: epic-expanded-harness-support
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-13
---

# Typed Target Registry and Adapter Contract

## Brief

Replace the closed Codex/Claude target enumerations with one typed registry that
drives harness policy, CLI validation and help, enabled-target resolution,
adapter composition, scoped capability profiles, observation dispatch, and
status rendering. Existing generic `HarnessId`, inventory, target-local state,
and output contracts remain the domain foundation rather than being replaced
with a new hierarchy.

Define the reusable adapter acceptance contract alongside the registry so each
target supplies its own documented paths, codecs, probes, reload behavior, and
native lifecycle capabilities through the same bounded ports. Test support must
derive isolated roots and fake executable profiles from this registry instead
of adding another hard-coded branch for every harness. This feature does not
implement the individual target adapters.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: foundation feature; managed projection and every concrete
  adapter depend on its target and acceptance contracts.

## Simplification opportunity

- Remove repeated Codex/Claude matches from configuration, CLI parsing,
  application composition, status observation, and fixtures while preserving
  the first-party plugin bootstrap as its intentionally narrower distribution
  surface.

## Foundation references

- `docs/SPEC.md` — Harness, Operating Model, Configuration Directory.
- `docs/ARCH.md` — Harness Adapter Contract, Capability Detection, Testing.
- `docs/UX.md` — Target and Scope, Common Flags, Help and Diagnostic Discovery.
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Expanded Target Set.

## Grounding summary

Probed the closed enumerations this feature replaces (verified, not assumed):

- `HarnessKind { Codex, Claude }` and the `select_profile` / `compiled_capabilities`
  / `unknown_capabilities` free functions in `crates/harnesses/src/lib.rs` are the
  adapter-dispatch and capability-profile closed surface.
- `HarnessPolicies { codex: HarnessPolicy, claude: HarnessPolicy }` in
  `crates/core/src/storage/config.rs` is the closed config shape; it serializes as
  `[harnesses.codex]` / `[harnesses.claude]` TOML tables.
- `parse_harness` / `parse_target` in `crates/cli/src/command.rs` hardcode the
  accepted id literals.
- `enabled_harnesses` in `crates/cli/src/application.rs` hardcodes the enabled
  list; `instruction_locations`, `skill_destination`, `configured_native_profile`,
  and `lifecycle_preview_presence` re-match on `"codex"` / `"claude"` strings.
- `FakeNativeMode::CodexVersion` / `FakeNativeMode::ClaudeVersion` in
  `crates/test-support/src/native_process.rs` are the per-harness fixture branch.

Existing generic foundations are retained, not replaced: `HarnessId`
(`crates/core/src/domain/identity.rs`) is already an opaque validated newtype that
accepts every intended identifier (`pi`, `gemini`, `kimi`, ...); `HarnessSet` /
`TargetSelection` / `resolve_targets` (`crates/core/src/domain/scope.rs` and
`crates/core/src/runtime/scope.rs`) already resolve targets generically against an
enabled set; `CapabilityProfileSelection` and `ScopedCapabilitySets`
(`crates/core/src/domain/installation.rs`) already encode the verified-compiled /
observe-only authority model. The work is to feed these generic types from one
authoritative registry instead of from re-derived Codex/Claude literals.

Foundation docs already describe the intended future state (SPEC Terminology
lists the expanded ids; HARNESS-CONTRACTS Expanded Target Set enumerates them;
ARCH Harness Adapter Contract already specifies the trait shape; UX Target and
Scope already shows `--target gemini`). This feature is code-first against those
already-rolled-forward assertions; no foundation-doc edits are required at design
time.

## Design decisions

- **Single source of truth shape**: one `TargetRegistry` constructed in
  `skilltap-harnesses`, exposing `&dyn HarnessAdapter` per `HarnessId`. Chosen
  over a closed `AdapterFamily` enum (which would re-introduce the closed
  enumeration this feature removes) and over a runtime submission mechanism like
  `inventory` (unnecessary while every adapter lives in one crate; revisited if
  adapters ever split into per-crate plugins — see Risks).
- **Dependency direction preserved**: the registry and adapter trait live in
  `skilltap-harnesses` because core must not depend on concrete adapters. Core's
  config type becomes a `HarnessPolicyMap` keyed by the generic `HarnessId`; it
  validates structure only. Id *membership* ("is `gemini` a real target?") is
  enforced at the CLI composition boundary where the registry is in hand, not in
  core. This keeps the Single Source of Truth in harnesses without leaking
  adapter identity into core.
- **Config wire compatibility**: `HarnessPolicies { codex, claude }` is replaced
  by a `BTreeMap<HarnessId, HarnessPolicy>`. TOML serializes a struct field and a
  map entry to the same `[harnesses.<id>]` table, so existing `config.toml` files
  with `[harnesses.codex]` / `[harnesses.claude]` round-trip unchanged and
  `CONFIG_SCHEMA_VERSION` stays at 1. This is the lowest-risk migration path and
  avoids destabilizing the in-flight `3.0.0` release.
- **Distinct adapters retained**: each target implements `HarnessAdapter` as its
  own struct (`CodexAdapter`, `ClaudeAdapter`). The trait exposes bounded ports;
  orchestration in the CLI never re-matches on the id string. Native codecs,
  probes, paths, and lifecycle vectors stay adapter-private. The registry is a
  dispatch and metadata table, not a universal plugin format.
- **Capability profiles stay adapter-owned**: `select_profile` /
  `compiled_capabilities` / `unknown_capabilities` move verbatim into
  `CodexAdapter::select_profile` and `ClaudeAdapter::select_profile`. Verified
  compiled profiles remain the only mutation authority, exactly as today.
- **First-party plugin bootstrap stays narrow**: `DistributionSurface::
  FirstPartyPlugin` is adapter metadata set only on Codex and Claude, encoding
  the epic's strategic decision that the self-hosted plugin is a Codex/Claude
  distribution surface and nothing else. Detection never implies bootstrap
  eligibility for other targets.
- **CLI help is registry-derived**: `--target` and the `harness enable`/
  `disable` positionals derive their accepted values and help text from
  `TargetRegistry::ids()`. `parse_harness`/`parse_target` continue to return a
  typed `HarnessId`/`TargetSelection` for any structurally valid id; the dispatch
  layer performs registry membership validation and emits a typed
  `target_not_registered` error so a typo never silently enables an unknown id.
- **Test fixtures derived from the registry**: a `FakeHarnessProfile` is built
  from a `TargetDescriptor`; adding a target adds a profile constructor, not a
  new `FakeNativeMode` variant. The generic process-behavior modes (`Hang`,
  `Flood`, `ProbeNarrow`, ...) stay in `FakeNativeMode` since they are orthogonal
  to harness identity.
- **No speculative adapters**: this feature defines the contract and migrates
  Codex/Claude onto it. The eleven direct targets, Pi, and the boundary
  candidates are out of scope; their adapter features register descriptors here.

## Architectural choice

**Chosen**: a compile-time `TargetRegistry` of `&'static dyn HarnessAdapter`
entries, built in `skilltap-harnesses` via a `TargetRegistry::canonical()`
constructor. Adding a target is two edits in one crate: a new adapter module and
one entry in `canonical()`. The registry is the single derivation point for id
sets (CLI help, validation), enabled resolution (config map), adapter dispatch
(observation, lifecycle, instruction, skill projection), capability selection,
and the test profile catalog.

**Rejected — closed `AdapterFamily` enum**: dispatching on a `CodexLike` /
`ClaudeLike` / `FileManaged` family would re-open a closed enumeration on every
new adapter and force unrelated adapters to share a family bucket, contradicting
the brief's "distinct native adapters" requirement.

**Rejected — runtime `inventory` submission**: every adapter currently lives in
`skilltap-harnesses`, so a const registry in that crate captures the same
single-source-of-truth property without a new dependency or a registration
side-effect. If adapters ever split into per-crate plugins, this decision is the
atural revision point (see Risks).

## Implementation Units

### Unit 1: Target registry and adapter contract

**File**: `crates/harnesses/src/registry.rs` (new); re-exports added to
`crates/harnesses/src/lib.rs`.

**Story**: `epic-expanded-harness-support-registry-contract`.

```rust
use std::ffi::OsString;

use skilltap_core::domain::{
    CapabilityProfileSelection, HarnessId, NativeVersion, Scope,
};
use skilltap_core::runtime::{ObservationRuntimeError, PlatformPaths};

use crate::adapters::{ClaudeAdapter, CodexAdapter};
use crate::lifecycle::NativeLifecycleRequest;
use crate::{CanonicalObservation, DetectionError};

/// Whether a target participates in skilltap's self-hosted first-party plugin
/// bootstrap. Only Codex and Claude per the epic's strategic decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistributionSurface {
    FirstPartyPlugin,
    Managed,
}

/// Stable identity and display metadata for one registered target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetIdentity {
    pub id: HarnessId,
    pub display_name: &'static str,
    pub distribution_surface: DistributionSurface,
}

/// Documented native observation roots for one concrete scope. Adapters return
/// only the roots they own; the orchestrator never reads arbitrary paths.
#[derive(Clone, Debug)]
pub struct AdapterObservationPaths {
    pub canonical: Vec<CanonicalObservation>,
    pub project_entry_count: Option<usize>,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ObservationPathError {
    #[error(transparent)]
    Validation(#[from] skilltap_core::domain::ValidationError),
    #[error(transparent)]
    Runtime(#[from] ObservationRuntimeError),
}

/// One registered target adapter. Codex and Claude migrate onto this; new
/// adapter features add their own implementations. The trait exposes bounded
/// ports; CLI orchestration operates purely through this trait and never
/// re-matches on the harness id string.
pub trait HarnessAdapter: Sync {
    fn identity(&self) -> TargetIdentity;

    // --- Detection (required) ---
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;

    // --- Capability profile (required) ---
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;

    // --- Observation (required) ---
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: skilltap_core::runtime::ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;

    // --- Optional ports; default absent. Adapters implement the subset the
    //     target actually supports, mirroring the capability model. ---
    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        None
    }
    fn instruction_bridge(&self) -> Option<&dyn InstructionBridgePort> {
        None
    }
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        None
    }
}

/// Native marketplace/plugin lifecycle argument vector for one request.
/// Replaces the `HarnessKind`-matched free function `native_arguments`.
pub trait NativeLifecycleVector: Sync {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, crate::lifecycle::NativeLifecycleError>;
}

/// Harness-native instruction bridge location for one scope.
pub trait InstructionBridgePort: Sync {
    fn global_bridge(&self, paths: &PlatformPaths) -> Option<skilltap_core::domain::AbsolutePath>;
    fn project_bridge(
        &self,
        project: &skilltap_core::domain::AbsolutePath,
    ) -> Option<skilltap_core::domain::AbsolutePath>;
}

/// Where skilltap projects a standalone skill for this target.
pub trait SkillProjectionPort: Sync {
    fn destination(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
    ) -> Option<skilltap_core::domain::AbsolutePath>;
}

/// The authoritative typed target registry.
#[derive(Clone, Debug)]
pub struct TargetRegistry {
    adapters: Vec<&'static dyn HarnessAdapter>, // stable insertion order
}

impl TargetRegistry {
    /// Canonical registry. Adding a target = one adapter module + one entry
    /// here. No closed enum is reopened anywhere else.
    pub fn canonical() -> Self {
        Self {
            adapters: vec![
                CodexAdapter::static_ref(),
                ClaudeAdapter::static_ref(),
            ],
        }
    }

    pub fn contains(&self, id: &HarnessId) -> bool {
        self.adapters.iter().any(|a| &a.identity().id == id)
    }

    pub fn adapter(&self, id: &HarnessId) -> Option<&'static dyn HarnessAdapter> {
        self.adapters.iter().copied().find(|a| &a.identity().id == id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &HarnessId> {
        self.adapters.iter().map(|a| &a.identity().id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static dyn HarnessAdapter> {
        self.adapters.iter().copied()
    }

    pub fn first_party_targets(&self) -> impl Iterator<Item = &'static dyn HarnessAdapter> {
        self.adapters.iter().copied().filter(|a| {
            a.identity().distribution_surface == DistributionSurface::FirstPartyPlugin
        })
    }
}
```

**Implementation Notes**:

- Adapters are stateless singletons; all per-call state (paths, scope,
  environment, limits) arrives through method arguments, so `&'static dyn
  HarnessAdapter` is sound.
- `HarnessKind` is fully eliminated: every site that matched on `HarnessKind::
  Codex` / `Claude` dispatches through `registry.adapter(&id)` instead.
- `CodexAdapter::select_profile` / `ClaudeAdapter::select_profile` reproduce the
  existing `select_profile(harness, version)` logic byte-for-byte, including the
  `codex-0-144-1` / `claude-2-1-201` profile ids, the version equality check, the
  asymmetric plugin.update/marketplace/project capability matrix, and the
  observe-only `unknown_capabilities` fallback.

**Acceptance Criteria**:

- [ ] `TargetRegistry::canonical().ids()` yields exactly `codex` and `claude`.
- [ ] `first_party_targets()` yields exactly `codex` and `claude`.
- [ ] `adapter(&HarnessId::new("gemini").unwrap())` returns `None` (no
      speculative adapters registered).
- [ ] For the verified Codex and Claude versions, `adapter.select_profile(&
      version)` returns a `VerifiedCompiled` selection identical to today's
      `select_profile` output (asserted by a table test mirroring the existing
      capability matrix).
- [ ] For an unknown version, `select_profile` returns `UnknownVersion` with the
      same unverified capability set as today.

---

### Unit 2: Codex and Claude adapter migration

**Files**: `crates/harnesses/src/adapters/mod.rs`,
`crates/harnesses/src/adapters/codex.rs`,
`crates/harnesses/src/adapters/claude.rs` (all new).

**Story**: `epic-expanded-harness-support-registry-adapters`.

```rust
// crates/harnesses/src/adapters/codex.rs
use std::ffi::OsString;

use skilltap_core::domain::{CapabilityProfileSelection, NativeVersion, Scope};
use skilltap_core::runtime::PlatformPaths;

use crate::adapter_helpers; // existing observe_codex_* free functions stay
use crate::lifecycle::{NativeLifecycleError, NativeLifecycleRequest};
use crate::registry::{
    AdapterObservationPaths, DetectionError, DistributionSurface, HarnessAdapter,
    InstructionBridgePort, NativeLifecycleVector, SkillProjectionPort, TargetIdentity,
};
use crate::{codex_lifecycle_arguments, CodexInstructionBridge, CodexSkillProjection};

pub struct CodexAdapter;

impl CodexAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &Self
    }
}

impl HarnessAdapter for CodexAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("codex").expect("static harness id"),
            display_name: "Codex",
            distribution_surface: DistributionSurface::FirstPartyPlugin,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError> {
        adapter_helpers::decode_codex_version(stdout)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_codex_profile(version) // moved verbatim
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: skilltap_core::runtime::ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, crate::registry::ObservationPathError> {
        adapter_helpers::observe_codex(paths, scope, limits)
    }

    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        Some(&CodexLifecycle)
    }
    fn instruction_bridge(&self) -> Option<&dyn InstructionBridgePort> {
        Some(&CodexInstructionBridge)
    }
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&CodexSkillProjection)
    }
}
```

`ClaudeAdapter` mirrors this with `"claude"` / `"Claude Code"`, the
`2.1.201 (Claude Code)` suffix decode, and Claude's capability matrix.

**Implementation Notes**:

- `crates/harnesses/src/lib.rs` keeps the existing free functions
  (`observe_codex_canonical_resources`, `observe_claude_canonical_resources`,
  `decode_native_version`, `select_profile`, `compiled_capabilities`, etc.) but
  relocates them into a private `adapter_helpers` module. The adapter structs
  delegate to them unchanged, so Codex/Claude behavior is byte-identical.
- `NativeLifecycleRequest` drops its `harness: HarnessKind` field. The request
  already carries the action, scope, name, and source; the owning adapter's
  `NativeLifecycleVector::arguments` supplies the per-harness vector. The CLI no
  longer constructs `HarnessKind` from a string match.
- The Codex project-scope unsupported constraint (today's
  `HarnessKind::Codex if project => Err(UnsupportedProjectScope)`) moves into
  `CodexLifecycle::arguments` unchanged.

**Acceptance Criteria**:

- [ ] Every existing Codex/Claude detection, capability, observation, lifecycle,
      instruction, and skill-projection test passes without modification to its
      assertions.
- [ ] `git grep -n "HarnessKind" crates/` returns no matches after migration.
- [ ] `git grep -n '"codex"\|"claude"' crates/cli/src/` returns no match arms
      that dispatch behavior (only display labels remain, if any).

---

### Unit 3: Configuration validation as a registry-driven map

**File**: `crates/core/src/storage/config.rs` (modified).

**Story**: `epic-expanded-harness-support-registry-config`.

```rust
// Replaces HarnessPolicies { codex, claude }. Wire-compatible TOML map.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HarnessPolicyMap(BTreeMap<HarnessId, HarnessPolicy>);

impl HarnessPolicyMap {
    pub fn get(&self, id: &HarnessId) -> Option<&HarnessPolicy>;
    pub fn iter(&self) -> impl Iterator<Item = (&HarnessId, &HarnessPolicy)> + '_;
    /// Enabled ids in stable id order; replaces the hardcoded list in
    /// application.rs::enabled_harnesses.
    pub fn enabled(&self) -> impl Iterator<Item = &HarnessId> + '_;
    pub fn with_policy(
        &self,
        id: HarnessId,
        enabled: bool,
        binary: Option<&HarnessBinary>,
    ) -> Self;
}

// ConfigDocument becomes:
pub struct ConfigDocument {
    harnesses: HarnessPolicyMap, // was HarnessPolicies
    instructions: InstructionPolicy,
    updates: UpdatePolicy,
    bootstrap: BinaryUpdatePolicy,
}

impl ConfigDocument {
    /// Seeds codex and claude disabled with PATH-lookup binaries. Unchanged
    /// from today's defaults() output.
    pub fn defaults() -> Self { /* codex + claude, both disabled */ }

    /// Works for any HarnessId. Membership in the registry is enforced by the
    /// CLI composition layer (see Unit 5), not here.
    pub fn with_harness_policy(
        &self,
        harness: &HarnessId,
        enabled: bool,
        binary: Option<&HarnessBinary>,
    ) -> Result<Self, SchemaError> { /* insert/update the map entry */ }
}
```

**Implementation Notes**:

- `HarnessPolicyMap` serializes as `[harnesses.<id>]` tables — identical TOML to
  today's struct for `codex` and `claude`, so `config.toml` files round-trip
  unchanged and `CONFIG_SCHEMA_VERSION` stays at 1.
- `HarnessPolicy` keeps `deny_unknown_fields` per entry, preserving the current
  rejection of unknown keys within a harness table.
- Core validates structure only (valid `HarnessId`, valid `HarnessBinary`). Id
  membership is a composition-layer concern so core stays adapter-free.
- `defaults()` still seeds exactly `codex` and `claude` (both disabled) so the
  first-use behavior described in SPEC/UX is unchanged.

**Acceptance Criteria**:

- [ ] A `config.toml` containing only `[harnesses.codex]` and `[harnesses.claude]`
      deserializes to a `ConfigDocument` equal to today's output.
- [ ] Round-trip: `to_string` then `parse` is identity for the defaults document
      and for a two-harness enabled document.
- [ ] `with_harness_policy(&HarnessId::new("gemini").unwrap(), true, None)`
      succeeds at the config layer (the registry rejects it at composition).
- [ ] `HarnessPolicyMap::enabled()` yields `codex` and `claude` in id order when
      both are enabled, matching today's `enabled_harnesses`.

---

### Unit 4: CLI parser, help, and composition dispatch

**Files**: `crates/cli/src/command.rs`, `crates/cli/src/entrypoint.rs`,
`crates/cli/src/application.rs` (and submodules under
`crates/cli/src/application/`).

**Story**: `epic-expanded-harness-support-registry-cli`.

```rust
// crates/cli/src/command.rs

/// Parses any structurally valid HarnessId. Registry membership is enforced
/// in the dispatch layer where TargetRegistry is in hand.
fn parse_harness(value: &str) -> Result<HarnessId, String> {
    HarnessId::new(value).map_err(|error| error.to_string())
}

fn parse_target(value: &str) -> Result<TargetSelection, String> {
    match value {
        "all" => Ok(TargetSelection::All),
        _ => parse_harness(value).map(TargetSelection::Only),
    }
}

// crates/cli/src/entrypoint.rs
use skilltap_harnesses::TargetRegistry;

/// Built once from the canonical registry; threaded into help and dispatch.
struct Composition {
    registry: TargetRegistry,
    // ...existing composition state
}

impl Composition {
    fn build() -> Self {
        Self { registry: TargetRegistry::canonical(), /* ... */ }
    }

    /// Augment clap's --target / harness positional help from the registry so
    /// `--help` enumerates registered harnesses without a hardcoded list.
    fn augment_help(command: clap::Command, registry: &TargetRegistry) -> clap::Command {
        let ids = registry.ids().map(HarnessId::as_str).collect::<Vec<_>>().join("|c");
        command.mut_arg("target", |arg| arg.help(
            format!("Select one registered harness ({ids}) or every enabled harness"))
        )
    }

    /// Membership validation at the composition boundary.
    fn validate_target(&self, id: &HarnessId) -> Result<(), ErrorDetail> {
        if self.registry.contains(id) {
            Ok(())
        } else {
            Err(ErrorDetail::new(
                "target_not_registered",
                "The requested harness is not registered in this build.",
            ))
            .with_context("harness", id.as_str())
        }
    }
}
```

In `crates/cli/src/application.rs`, the per-target string matches are replaced by
registry dispatch:

```rust
// was: match target.as_str() { "codex" => ..., "claude" => ..., _ => return None }
fn instruction_locations(
    registry: &TargetRegistry,
    paths: &PlatformPaths,
    scope: &Scope,
    enabled: &[HarnessId],
) -> (AbsolutePath, Vec<(HarnessId, AbsolutePath)>) {
    let canonical = paths.global_agents().clone(); // always ~/AGENTS.md
    let bridges = enabled
        .iter()
        .filter_map(|id| {
            let adapter = registry.adapter(id)?;
            let port = adapter.instruction_bridge()?;
            let bridge = match scope {
                Scope::Global => port.global_bridge(paths),
                Scope::Project(project) => port.project_bridge(project),
            }?;
            Some((id.clone(), bridge))
        })
        .collect();
    (canonical, bridges)
}
```

`skill_destination`, `configured_native_profile`, `lifecycle_preview_presence`,
and the lifecycle `HarnessKind` mapping are converted the same way: each asks
`registry.adapter(&id)` and calls the relevant port.

`enabled_harnesses(config)` becomes `config.harnesses().enabled().cloned().collect()`
against the new map.

**Implementation Notes**:

- `--target`/positional help is registry-derived so `skilltap --help` lists
  exactly the registered harnesses with no hardcoded literals.
- The `bootstrap` command's `--target` accepts only `FirstPartyPlugin` targets;
  the dispatch layer filters `registry.first_party_targets()`, preserving the
  narrow Codex/Claude bootstrap surface.
- `detection_diagnostic` and the existing `no_enabled_harnesses` /
  `target_not_enabled` next-action messages are unchanged except that
  `skilltap harness enable <codex|claude>` becomes
  `skilltap harness enable <registered-harness>` derived from the registry.

**Acceptance Criteria**:

- [ ] `skilltap --help` lists registered harnesses without any hardcoded id
      string in the rendering path.
- [ ] `skilltap harness enable gemini` (not yet registered) fails with
      `target_not_registered` at the composition boundary, never writing config.
- [ ] `skilltap harness enable codex` and `... claude` behave exactly as today
      (existing compiled-binary tests pass unchanged).
- [ ] `skilltap bootstrap --target codex` and `... claude` succeed; any other id
      is rejected because it is not a `FirstPartyPlugin` target.
- [ ] `--target all` expands to every enabled registered harness from the map.

---

### Unit 5: Reusable acceptance-test contract

**Files**: `crates/test-support/src/harness_profile.rs` (new),
`crates/test-support/src/native_process.rs` (modified),
`crates/test-support/src/lib.rs` (re-export).

**Story**: `epic-expanded-harness-support-registry-test-support`.

```rust
// crates/test-support/src/harness_profile.rs

/// How a fake harness responds to its version probe. Replaces the
/// FakeNativeMode::CodexVersion / ClaudeVersion variants.
#[derive(Clone, Debug)]
pub enum VersionResponse {
    TextPrefix { prefix: &'static str, version: &'static str }, // codex-cli 0.144.1
    TextSuffix { version: &'static str, suffix: &'static str }, // 2.1.201 (Claude Code)
    Json { version: &'static str },                              // {"version":"..."}
}

/// Which native lifecycle dialect the fake executable emulates, if any.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleDialect {
    Codex,  // plugin add/remove, marketplace add/remove/upgrade
    Claude, // plugin install/uninstall/update, marketplace add/remove/update
    None,
}

/// A fake-harness profile derived from one TargetDescriptor. Adding a target
/// adds a constructor here, not a new FakeNativeMode variant.
#[derive(Clone, Debug)]
pub struct FakeHarnessProfile {
    pub id: HarnessId,
    pub version_response: VersionResponse,
    pub lifecycle_dialect: LifecycleDialect,
}

impl FakeHarnessProfile {
    pub fn codex() -> Self { /* TextPrefix + Codex dialect */ }
    pub fn claude() -> Self { /* TextSuffix + Claude dialect */ }

    /// Materialize a fake executable for this profile under an isolated root,
    /// composing the generic process-behavior mode (Hang, Flood, ...) with the
    /// harness-specific version/lifecycle script.
    pub fn build(
        &self,
        root: &Path,
        behavior: FakeNativeMode,
    ) -> io::Result<FakeNativeProcess>;
}

/// The shared acceptance matrix every registered adapter must pass: detection,
/// both scopes, complete skills, MCP observation, reload, drift, removal, and
/// immediate-repeat idempotency. Adapter features invoke this with their own
/// profile; this feature populates it for Codex and Claude.
pub fn acceptance_matrix(
    profile: &FakeHarnessProfile,
    machine: &IsolatedMachine,
) -> AcceptanceReport;
```

**Implementation Notes**:

- `FakeNativeMode::CodexVersion` and `FakeNativeMode::ClaudeVersion` are
  removed; their byte output is reproduced exactly by
  `FakeHarnessProfile::codex().build(root, FakeNativeMode::VersionKnown)` and
  `::claude()`. The generic modes (`Hang`, `Flood`, `ProbeNarrow`, `ProbeDrift`,
  `MalformedJson`, `DuplicateJson`, `ExtraJsonDocument`, `RetainPipes`, `Exit`)
  stay because they describe process behavior orthogonal to harness identity.
- The lifecycle script block currently gated by `matches!(mode, CodexVersion |
  ClaudeVersion | VersionKnown)` moves behind `LifecycleDialect`, so any future
  adapter whose dialect is Codex-like or Claude-like reuses it without a new
  branch.
- `acceptance_matrix` codifies the HARNESS-CONTRACTS "Adding Another Harness"
  criteria as a reusable routine. This feature runs it for Codex and Claude;
  adapter features run it for their own profiles.

**Acceptance Criteria**:

- [ ] `FakeHarnessProfile::codex().build(root, VersionKnown)` produces an
      executable whose `--version` output is byte-identical to today's
      `FakeNativeMode::CodexVersion`.
- [ ] Same for `claude()` vs the removed `ClaudeVersion` variant.
- [ ] Every existing test that constructed a Codex or Claude fake passes after
      migrating to the profile constructor.
- [ ] `acceptance_matrix(&FakeHarnessProfile::codex(), machine)` passes the full
      detection/scope/skill/mcp/drift/removal/idempotency suite, and likewise
      for `claude()`.

---

## Implementation Order

1. `epic-expanded-harness-support-registry-contract` (Unit 1, registry +
   adapter trait) — `depends_on: []`. Foundation story; everything else binds
   to its trait surface. Terminalizes independently of the parent feature so
   its sibling stories can become ready.
2. `epic-expanded-harness-support-registry-adapters` (Unit 2, Codex/Claude
   migration) — `depends_on: [epic-expanded-harness-support-registry-contract]`.
   Makes the contract concrete and proves behavior is preserved.
3. `epic-expanded-harness-support-registry-config` (Unit 3, config map) —
   `depends_on: [epic-expanded-harness-support-registry-contract]`.
   Wire-compatible; enables enabled-resolution and membership validation.
4. `epic-expanded-harness-support-registry-cli` (Unit 4, parser/help/dispatch) —
   `depends_on: [epic-expanded-harness-support-registry-contract,
   epic-expanded-harness-support-registry-adapters,
   epic-expanded-harness-support-registry-config]`. The composition boundary that
   ties registry + config together.
5. `epic-expanded-harness-support-registry-test-support` (Unit 5, acceptance
   contract) — `depends_on: [epic-expanded-harness-support-registry-contract,
   epic-expanded-harness-support-registry-adapters]`. Reusable matrix; exercises
   the registry-driven fixtures.

The parent feature `epic-expanded-harness-support-registry` carries the design
body only; it has no `depends_on` and is never an inline stride. Its five child
stories carry Units 1–5 respectively. Sibling adapter features outside this
registry subtree continue to depend on the parent feature id (the whole
registry deliverable), which is correct: they wait for the full contract +
Codex/Claude migration + config + CLI composition to terminalize together,
which the parent feature reaches once all five children are done.

## Simplification

- **Eliminate** `HarnessKind` (closed adapter-dispatch enum).
- **Eliminate** `HarnessPolicies { codex, claude }` (closed config struct) in
  favor of `HarnessPolicyMap`.
- **Eliminate** the `parse_harness` / `parse_target` hardcoded id literals;
  replace with structural parse plus registry membership validation.
- **Eliminate** the `enabled_harnesses` hardcoded list.
- **Eliminate** `FakeNativeMode::CodexVersion` / `ClaudeVersion`.
- **Relocate** (not duplicate) `select_profile`, `compiled_capabilities`,
  `unknown_capabilities`, `decode_native_version`, and the `observe_*` free
  functions into adapter modules / `adapter_helpers`.
- **Eliminate** every behavior-dispatching `match target.as_str()` in
  `crates/cli/src/application.rs` and its submodules (instruction, skill,
  lifecycle, profile preview).
- **Retain intentionally**: the narrow first-party plugin bootstrap surface
  (Codex/Claude only) as `DistributionSurface::FirstPartyPlugin` metadata; the
  distinct Codex and Claude observation/lifecycle codecs; the generic
  `HarnessId`, `HarnessSet`, `TargetSelection`, and `resolve_targets` types.

No separate `[refactor]` / `[cleanup]` child story is warranted: every
elimination is bound to the unit that introduces its replacement, and each is
independently reviewable as part of that story.

## Testing

- **Interface tests (Unit 1)**: `TargetRegistry::canonical()` membership, id
  order, `first_party_targets` filter, and `adapter(id)` lookup. Protects the
  single-source-of-truth contract.
- **Regression tests (Unit 2)**: a capability-matrix table test asserting
  `CodexAdapter`/`ClaudeAdapter` reproduce today's `select_profile` output for
  the verified version and for an unknown version. Protects the no-behavior-
  change guarantee.
- **Wire-compatibility test (Unit 3)**: parse a v1 `config.toml` with
  `[harnesses.codex]`/`[harnesses.claude]` and assert equality with today's
  `ConfigDocument`; round-trip defaults and a two-enabled document. Protects the
  schema-stays-at-1 invariant and the in-flight `3.0.0` release.
- **Membership-validation test (Unit 4)**: `harness enable <unregistered>` fails
  with `target_not_registered` and writes nothing; `--target all` expands from
  the map. Protects the composition-boundary contract.
- **Acceptance matrix (Unit 5)**: `FakeHarnessProfile::codex()`/`claude()` run
  the full detection/scope/skill/mcp/drift/removal/idempotency suite. Protects
  the reusable contract adapter features will rely on.
- **Removals**: assertions previously pinning `HarnessKind`, the `HarnessPolicies`
  field shape, or the removed `FakeNativeMode` variants are updated in the same
  story that removes the type; no tautological replacement is introduced.

Low-value tests are not added: no per-adapter unit test for `version_arguments`
  (it is one literal), no exhaustive `HarnessPolicyMap` iteration test (the
  interface test covers the contract), and no separate test for help-string
  formatting beyond one assertion that a registered id appears in `--help`.

## Implementation discovery

### Initial defect (resolved 2026-07-12)

The initial decomposition made Unit 1 an inline parent-feature stride while
making every child story `depends_on` the parent feature id. Since readiness
is governed by `depends_on` reaching a terminal stage and the parent feature
cannot terminalize until its children complete, that graph left every child
blocked forever: the four children all pointed at a nonterminal ancestor, and
the ancestor could not finish before the children that depended on it.

### Correction

Unit 1 is now its own foundation story,
`epic-expanded-harness-support-registry-contract` (`depends_on: []`, sibling to
Units 2–5 under the same parent feature). Every child story's `depends_on`
now points at that foundation story (and, for cli/test-support, at sibling
stories) — never at the parent feature id. The parent feature keeps the design
body and no `depends_on`; it terminalizes once all five children are done,
which also unblocks the sibling adapter features that legitimately wait on the
whole registry deliverable.

The substantive architecture is unchanged: the trait shape, the registry API,
the wire-compatible config map, the registry-derived CLI, and the
registry-derived test fixtures are all as designed. Only the work graph was
restructured to be executable.

## Risks

- **Riskiest assumption**: that the config map migration is wire-compatible and
  does not require a schema bump or a migration step. Mitigation: the Unit 3
  round-trip test pins this before any CLI change. If TOML map serialization
  diverges from struct serialization in any toolchain version, the fallback is a
  schema bump to 2 with an explicit one-time loader that accepts the legacy
  two-key form — the design does not assume this fallback is needed.
- **`3.0.0` in quality gate**: this feature targets `main` after `3.0.0` ships.
  The config change is wire-compatible and additive, so it must not be
  cherry-picked onto the release branch. Story 3 calls this out in its body.
- **Trait sprawl**: `HarnessAdapter` plus three optional port traits could grow
  as adapter features land. Mitigation: optional capabilities stay behind
  `Option<&dyn Port>` accessors returning `None` by default, so a file-managed
  adapter implements only detection/observation/profile. If a fourth optional
  port appears, consider splitting `HarnessAdapter` into a required-core trait
  plus a capability-probe trait; deferred until earned.
- **Static singleton registry**: `&'static dyn HarnessAdapter` assumes adapters
  remain stateless and in-process. If adapters ever need per-process
  configuration (e.g., plugin discovery paths from config), the registry
  constructor takes that configuration and returns owned adapters. If adapters
  split into separately compiled plugins, revisit the rejected `inventory`
  submission design. Neither is needed for the in-scope Codex/Claude migration.
- **Help derivation**: `clap` derive-based help may resist runtime augmentation
  of possible-values. Fallback: keep derive for structure, override only the
  `help` text via `Command::mut_arg` at entrypoint build time. Exact rendered
  text is verified by one assertion, not by hand-maintained snapshots.

## Implementation summary (children complete)

All five child stories are `done` and reviewed, so the parent feature advances
`implementing → review` for its own feature review pass. This transition does
not approve the feature; it only signals that the contracted child scope is
complete and the parent is ready for feature-level review.

Realized architecture (matches the design above):

- Unit 1 (`registry-contract`, done): `TargetRegistry`, `HarnessAdapter`, and
  the optional port traits (`NativeLifecycleVector`, `InstructionBridgePort`,
  `SkillProjectionPort`) in `crates/harnesses/src/registry.rs`. Canonical
  registry yields exactly `codex` and `claude`; both are `FirstPartyPlugin`.
- Unit 2 (`registry-adapters`, done): `CodexAdapter` and `ClaudeAdapter`
  migrated onto the trait; capability matrices, observation codecs, lifecycle
  vectors, instruction bridges, and skill projection moved verbatim into
  adapter modules / `adapter_helpers`.
- Unit 3 (`registry-config`, done): `HarnessPolicies { codex, claude }`
  replaced by `HarnessPolicyMap(BTreeMap<HarnessId, HarnessPolicy>)`;
  schema-1 byte compatibility pinned by round-trip tests and the
  `stable_iter` ordering shim.
- Unit 4 (`registry-cli`, done): `TargetRegistry::canonical()` built once in
  `run_from`; help augmentation, target membership validation,
  `target_not_registered`, bootstrap first-party filtering, instruction
  bridges (including Claude alternates), skill destinations, lifecycle
  dispatch/revalidation, status labels, and first-use reporting all dispatch
  through adapter ports or adapter metadata. `HarnessKind`, the request
  target duplication, and every behavior-dispatching `match target.as_str()`
  in `crates/cli/src` are gone.
- Unit 5 (`registry-test-support`, done): `FakeHarnessProfile` plus the
  reusable `acceptance_matrix`; `FakeNativeMode::CodexVersion`/`ClaudeVersion`
  removed; fixture publication uses hard link on the same filesystem and a
  sealed-artifact symlink cross-device to avoid a copy-then-exec race.

Newly added adapter metadata / port methods (all adapter-private contract
extensions; no new behavior-dispatch list in CLI):

- `HarnessAdapter`: `decode_version_with_limits` (defaulted), `native_root`,
  `managed_project_lifecycle`, `bootstrap_next_action`,
  `bootstrap_capability_next_action`.
- `NativeLifecycleVector::observation_scope`.
- `InstructionBridgePort::alternate_project_bridges`.
- `AdapterObservationPaths::surface_labels`.
- `NativeLifecycleDispatch` (binds a semantic request to the selected
  `HarnessId` and lifecycle vector) and `NativeLifecyclePort::
  with_foreign_operations` for mixed native/managed plans.

Verification at the head of this transition:

- `cargo test --workspace --all-targets` — 558 passed across 18 suites.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo fmt --all -- --check`, `git diff --check` — clean.
- `git grep -n HarnessKind -- crates` — no matches.

Sibling adapter features that `depends_on: [epic-expanded-harness-support-
registry, ...]` are now unblocked at the dependency-graph level; they still
wait on their other declared dependencies (e.g. managed fallback parity)
before becoming ready.

