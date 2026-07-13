---
id: epic-expanded-harness-support-registry-contract
kind: story
stage: review
tags: []
parent: epic-expanded-harness-support-registry
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Target Registry and Adapter Contract

## Scope

Implement Unit 1 of the registry feature design: the authoritative typed
`TargetRegistry` and the `HarnessAdapter` trait (plus its three optional port
traits) that every concrete adapter implements. This story is the foundation
the other four registry child stories bind to: adapters migrate onto the trait,
config keys off the trait's `HarnessId`, CLI dispatches through the registry,
and test-support derives profiles from a `TargetDescriptor`.

This story delivers the contract surface only. It registers no concrete adapter
beyond the `TargetRegistry::canonical()` constructor's two-entry literal list
(which Codex/Claude migration in `registry-adapters` populates); see Out of
scope. No `HarnessKind` elimination happens here — that is the adapters story's
job once the contract type exists.

## Units

- `crates/harnesses/src/registry.rs` (new): the types and trait below.
- `crates/harnesses/src/lib.rs` (modified): re-export the registry module and
  its public surface; the `CanonicalObservation`, `DetectionError`,
  `PlatformPaths`, `NativeLifecycleRequest`, and capability-profile types it
  references are already public.

```rust
use std::ffi::OsString;

use skilltap_core::domain::{
    CapabilityProfileSelection, HarnessId, NativeVersion, Scope,
};
use skilltap_core::runtime::{ObservationRuntimeError, PlatformPaths};

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

## Implementation notes

- Adapters are stateless singletons; all per-call state (paths, scope,
  environment, limits) arrives through method arguments, so `&'static dyn
  HarnessAdapter` is sound.
- This story introduces the trait and registry. It does not yet eliminate
  `HarnessKind` (the adapters story does) and does not yet define
  `CodexAdapter` / `ClaudeAdapter` (also the adapters story). To keep this
  story independently compilable, `TargetRegistry::canonical()`'s two adapter
  entries are added by `registry-adapters`; until then, this story lands the
  type definitions, the `TargetRegistry` methods, and an initially empty (or
  `todo!()`-guarded) `canonical()` whose constructor the adapters story fills.
  The interface tests below exercise the registry mechanics against a
  throwaway test adapter, not against the real Codex/Claude entries.
- `CodexAdapter::select_profile` / `ClaudeAdapter::select_profile` reproduce
  today's `select_profile(harness, version)` logic byte-for-byte — but that
  implementation lands in `registry-adapters`, not here. This story only
  declares the trait method.

### Completion

- Execution capability: highest, as directed by the autopilot caller because
  this contract becomes the architectural dispatch boundary for every target.
- Review weight: standard (caller/default).
- Files changed: `crates/harnesses/src/registry.rs`,
  `crates/harnesses/src/lib.rs`, and this story.
- Tests added/removed: added registry interface coverage for stable insertion
  order, lookup, membership, iteration, first-party filtering, the intentionally
  empty canonical registry, and default-absent optional ports; removed none.
- Simplification: kept `canonical()` empty and used one constructor seam for
  both production composition and throwaway test adapters, avoiding speculative
  Codex/Claude adapter shells or placeholder implementations.
- Discrepancies from design: privately cached each adapter's `TargetIdentity` so
  `ids()` can safely yield `&HarnessId` despite `identity()` returning an owned
  value; implemented `Debug` for the registry and the transparent error
  conversions manually because trait objects are not `Debug` and this crate does
  not directly depend on `thiserror`. The designed public contract is unchanged.
- Adjacent issues parked: none.
- Dispatch: direct-read only; the story is confined to one new module and one
  re-export boundary, and the caller prohibited delegation.
- Verification: `cargo test -p skilltap-harnesses --lib` passed 21 tests;
  `cargo check -p skilltap-harnesses`, the focused format check, and
  `git diff --check` passed.

## Acceptance criteria

- [ ] `TargetRegistry` exposes `contains`, `adapter`, `ids`, `iter`, and
      `first_party_targets` with the signatures above.
- [ ] `HarnessAdapter` declares the three required ports (`version_arguments`,
      `decode_version`, `select_profile`, `observe`) and three optional ports
      (`native_lifecycle`, `instruction_bridge`, `skill_projection`) defaulting
      to `None`.
- [ ] `NativeLifecycleVector`, `InstructionBridgePort`, and
      `SkillProjectionPort` are defined with the signatures above.
- [ ] An interface test constructs a `TargetRegistry` from a throwaway test
      adapter and asserts `ids()`, `contains`, `adapter(id)`, and the
      `first_party_targets` filter behave as specified.
- [ ] `crates/harnesses` compiles with the new module re-exported from
      `lib.rs`.

## Out of scope

- Codex/Claude adapter structs and `HarnessKind` elimination (Unit 2 /
  `registry-adapters`).
- The config map (Unit 3 / `registry-config`).
- CLI parser/help/dispatch (Unit 4 / `registry-cli`).
- The test-support acceptance contract and `FakeHarnessProfile` (Unit 5 /
  `registry-test-support`).
- Any new target adapter.
