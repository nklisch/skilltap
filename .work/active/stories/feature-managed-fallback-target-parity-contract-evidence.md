---
id: feature-managed-fallback-target-parity-contract-evidence
kind: story
stage: review
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

# Managed Projection Contract Evidence Amendment

## Scope

Amend the approved Unit 1 contract
(`feature-managed-fallback-target-parity-contract`, done at `caf5df03`) so the
Codex adapter can relocate onto the port without changing user-facing behavior
and without stringly CLI side channels. The amendment is a focused, additive-
where-possible revision of the public types in `skilltap-core` and
`skilltap-harnesses`; it does **not** reopen the approved contract story (it
stays `done` at its commit) and does **not** touch production Codex behavior.

The Codex adapter discovery
(`feature-managed-fallback-target-parity-codex-adapter` body) established four
evidence gaps in the approved contract. This story closes all four at the
contract layer:

1. **Caller-resolved confined source checkout.** The approved context carried a
   `SourceRevisionResolver`, whose only operation returns a `ResolvedRevision`.
   It cannot return the checked-out, confined source root that catalog/plugin
   readers need. The amendment introduces a `ResolvedSourceCheckout`
   (root + source + revision) that the orchestrator resolves once, reusing the
   existing `resolve_git_skill_source` machinery, and hands to the adapter. The
   `SourceRevisionResolver` trait stays revision-only and is removed from the
   managed-projection context.
2. **One authoritative marketplace checkout for fresh plugin installs.** A
   fresh plugin install carries no source on the plugin resource or
   `NativeLifecycleRequest`; the existing orchestrator resolves the selected
   marketplace source from inventory into `ResolvedSourceCheckout`. The
   adapter uses `checkout.source()` for provenance and reads from
   `checkout.root()`. No second source field can disagree with that checkout.
3. **Complete projection evidence.** The approved plan carried writes and
   omissions only. The adapter also produces the exact current aggregate
   fingerprint, desired aggregate fingerprint, and the complete
   `Vec<ManagedProjection>` manifest (Skill + Mcp per-surface fingerprints +
   Omitted). These drive ownership validation, drift detection, pending-attempt
   recovery, update-required checks, and persisted projection state. They
   cannot be reconstructed from `ManagedFileWrite` without parsing adapter-
   native documents in CLI. The amendment carries them in the plan.
4. **Removal without source acquisition.** Removal (plugin or marketplace)
   plans from the prior manifest plus current filesystem observation; it never
   needs source acquisition. The amendment models this directly: a
   `ManagedProjectionInput::Remove` variant carries no checkout, so removal
   cannot accidentally require source.

### Decision: collapse acquire/project into one `plan` method

The approved contract split `acquire`/`project`. The discovery shows the split
is artificial for this domain: the adapter loads catalog/plugin content and
maps it to target writes in one pass (the existing Codex
`plan_codex_component_projections` already does both), and removal skips
acquire entirely. Keeping two methods forces either an `Option<AcquiredProjection>`
that represents "this might be removal" (invalid state representable) or a
third `plan_removal` method (more surface).

The amendment collapses `acquire`/`project` into a single `plan` method taking
a `ManagedProjectionInput` enum. This is the shortest type-safe contract:
`Remove` carries no checkout (removal-without-source is unrepresentable as
anything else) and `Apply` always carries one. `AcquiredProjection` no longer
crosses the boundary (it becomes adapter-internal if the adapter wants it) and
is removed from the public contract.

### Decision: omit `omitted` field; manifest carries omissions

The approved plan had both `omitted: Vec<OmittedComponent>` and relied on the
orchestrator to fold omissions into the persisted manifest. With the adapter
now producing the complete manifest directly, a parallel `omitted` field would
allow the two to diverge (invalid state representable). The amendment removes
`omitted` and `OmittedComponent`; omissions live exclusively as
`ManagedProjection::Omitted` entries inside `plan.manifest`. The orchestrator's
defense-in-depth acknowledgment gate scans `manifest` for `Omitted` entries
when `acknowledged == false`.

Parent design: `feature-managed-fallback-target-parity` (Unit 1, as amended by
the Implementation discovery and contract amendment section).

## Units

- `crates/core/src/managed_projection.rs` (modified):
  - Add `ResolvedSourceCheckout { root, source, revision }` with `new`,
    `root`, `source`, `revision`.
  - Add `manifest: Vec<ManagedProjection>`, `current_fingerprint:
    Option<Fingerprint>`, `desired_fingerprint: Option<Fingerprint>` to
    `ManagedProjectionPlan`.
  - Remove `omitted: Vec<OmittedComponent>` from `ManagedProjectionPlan`.
  - Remove `OmittedComponent` and `AcquiredProjection` (and its
    `fingerprint`/`source`/`installed_revision` accessors).
  - `ManagedPluginWrite`, `ManagedFileWrite`, and `ManagedProjectionError` are
    unchanged (codes, summaries, `Other` discipline, and the existing test
    coverage all stay).
- `crates/harnesses/src/managed_projection.rs` (modified):
  - Add `ManagedProjectionInput<'a>` enum (`Apply { checkout }` | `Remove`).
  - Replace `ManagedAcquisitionContext` + `ManagedProjectionContext` with one
    `ManagedProjectionContext` carrying `input: ManagedProjectionInput<'a>`.
  - Replace `acquire` + `project` on `ManagedProjectionPort` with one `plan`
    method.
  - `ManagedLifecycleKind` is unchanged.
- `crates/harnesses/src/registry.rs`: no change. The accessor
  `managed_projection() -> Option<&dyn ManagedProjectionPort>` is unchanged
  (the trait is still `ManagedProjectionPort`; only its method set changed).
- Re-exports in `crates/core/src/lib.rs` and `crates/harnesses/src/lib.rs`:
  drop the removed names; add `ResolvedSourceCheckout` and
  `ManagedProjectionInput`.

### Exact target-neutral signatures

Core (`crates/core/src/managed_projection.rs`):

```rust
use crate::{
    domain::{AbsolutePath, Fingerprint, RelativeArtifactPath, ResolvedRevision, Source},
    runtime::DirectoryIdentity,
    storage::{ArtifactTree, ManagedProjection},
};

/// A confined source checkout the orchestrator resolved and handed to the
/// adapter for install/update projection. The adapter reads catalog/plugin
/// trees from `root`; it never re-implements git clone, fetch, or local-
/// source validation. Built by orchestrator-side source-resolution machinery
/// (the same machinery behind the existing `resolve_git_skill_source`),
/// not by `SourceRevisionResolver`, which stays revision-only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSourceCheckout {
    root: AbsolutePath,
    source: Source,
    revision: Option<ResolvedRevision>,
}

impl ResolvedSourceCheckout {
    /// Construct from already-validated components. Only orchestrator-side
    /// source resolvers should build this; adapters consume it read-only.
    pub fn new(
        root: AbsolutePath,
        source: Source,
        revision: Option<ResolvedRevision>,
    ) -> Self {
        Self { root, source, revision }
    }

    pub const fn root(&self) -> &AbsolutePath { &self.root }
    pub const fn source(&self) -> &Source { &self.source }
    pub const fn revision(&self) -> Option<&ResolvedRevision> { self.revision.as_ref() }
}

// ManagedPluginWrite and ManagedFileWrite are unchanged.

/// Pure target-bound writes plus complete projection evidence produced by an
/// adapter. The manifest and fingerprints drive ownership validation, drift
/// detection, pending-attempt recovery, update-required checks, and persisted
/// projection state; the orchestrator never reconstructs them from writes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ManagedProjectionPlan {
    pub trees: Vec<ManagedPluginWrite>,
    pub files: Vec<ManagedFileWrite>,
    /// Complete target-bound manifest including Skill and Mcp entries with
    /// per-surface fingerprints and Omitted entries the adapter classified
    /// (only when the orchestrator passed `acknowledged == true`; the
    /// orchestrator additionally defense-in-depth rejects any Omitted entry
    /// when `acknowledged == false`). The orchestrator sorts/dedups before
    /// persistence, matching the existing `PendingManagedAttempt` invariant.
    pub manifest: Vec<ManagedProjection>,
    /// Aggregate fingerprint of currently observed projected surfaces. None
    /// when nothing is currently projected (fresh install) or for removal of
    /// an absent surface. Drives drift detection and pending-attempt recovery.
    pub current_fingerprint: Option<Fingerprint>,
    /// Aggregate fingerprint of desired projected surfaces. None for removal
    /// or when the plan produces no desired surfaces. Drives update-required
    /// detection and the persisted ownership fingerprint.
    pub desired_fingerprint: Option<Fingerprint>,
}

// ManagedProjectionError is unchanged.
```

Harnesses (`crates/harnesses/src/managed_projection.rs`):

```rust
use skilltap_core::{
    domain::{AbsolutePath, HarnessId, ResourceKey, ResourceKind, Source},
    managed_projection::{
        ManagedProjectionError, ManagedProjectionPlan, ResolvedSourceCheckout,
    },
    runtime::{ConfinedFileSystem, JsonLimits, PlatformPaths},
    storage::ManagedProjection,
};

use crate::lifecycle::NativeLifecycleRequest;

// ManagedLifecycleKind is unchanged.

/// What the adapter plans. Invalid states are unrepresentable: Remove carries
/// no checkout; Apply always carries one.
#[derive(Clone, Debug)]
pub enum ManagedProjectionInput<'a> {
    /// Install or update. The orchestrator resolves the operation's single
    /// authoritative source into this checkout. For fresh plugin installs it
    /// resolves the selected marketplace source; the adapter reads the catalog
    /// and contained plugin tree from `checkout.root()` and records provenance
    /// from `checkout.source()`.
    Apply {
        checkout: &'a ResolvedSourceCheckout,
    },
    /// Remove. No source acquisition; the adapter plans exclusively from
    /// `prior` plus current filesystem observation of its own projected
    /// surfaces. This removes the approved contract's mandatory source
    /// acquisition for removal.
    Remove,
}

/// Inputs for one target-bound managed-projection plan.
pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub project: &'a AbsolutePath,
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

/// Target-specific acquisition and projection for the managed fallback
/// lifecycle. One planning method; the input enum distinguishes install/
/// update (with a caller-resolved checkout) from removal (no source). The
/// adapter owns catalog/plugin codec, target paths, and per-surface
/// fingerprint semantics; shared orchestration owns state, drift,
/// acknowledgment, publication, and load verification.
pub trait ManagedProjectionPort: Sync {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}
```

## Implementation notes

- This story amends types only; it does not migrate Codex behavior onto the
  port (the Codex adapter story does) and does not flip CLI dispatch (the
  orchestrator story does). `CodexAdapter` still does not override
  `managed_projection()` after this story, so production Codex behavior is
  unchanged.
- The orchestrator-side checkout resolver that produces
  `ResolvedSourceCheckout` is implemented by the orchestrator story
  (`feature-managed-fallback-target-parity-orchestrator`), reusing the existing
  `resolve_git_skill_source` machinery (local source → root = locator; git
  source → clone/fetch under `paths.skilltap_config()/managed/sources/`).
  This story only declares the type.
- The approved contract's interface test must be updated: it exercised
  `acquire`/`project` and `AcquiredProjection`. The amended test exercises the
  single `plan` method with both `ManagedProjectionInput::Apply` (round-tripping
  a checkout + plan with manifest and fingerprints) and
  `ManagedProjectionInput::Remove` (proving removal carries no checkout),
  asserting object safety and type round-trip as before.
- The approved contract's `ManagedProjectionError` table test is unchanged
  (the error model is unchanged).
- `ManagedProjection` is re-exported from `skilltap_core::storage` already; the
  manifest field references it. No new core dependency.
- Manual `Display`/`Error` impls continue to follow the `ObservationPathError`
  precedent (no `thiserror`).
- `ResolvedSourceCheckout` derives `Eq + PartialEq` so the interface test can
  compare values directly, matching the equality discipline the approved
  contract established for `AcquiredProjection`.

## Implementation discovery

Implementation stopped before code changes because the proposed `Apply` shape
still makes the selected marketplace checkout ambiguous for plugin operations:

- `ResolvedSourceCheckout` already owns the `Source` whose content exists at
  `root`; `checkout.source()` is therefore the source identity for that exact
  checkout.
- `ManagedProjectionInput::Apply` separately accepts
  `marketplace_source: Option<&Source>`. Nothing requires this second source to
  equal `checkout.source()`, and its optionality also permits a plugin Apply
  without the selected marketplace source the design says is required.
- The existing Codex flow has only one source at this boundary: it selects the
  marketplace `Source` from inventory/state, resolves that same source into a
  checkout, and reads the plugin as a contained path beneath that checkout
  (`crates/cli/src/application.rs`, the plugin branch of
  `plan_managed_codex_project_lifecycle`). The proposed contract turns that one
  identity into two independently supplied values.

A caller could therefore pass checkout A with marketplace source B. The adapter
would read bytes from A while using B for plugin source/provenance semantics,
with no type-level answer for which source is authoritative. That is the
ambiguous duplicate-source state the implementation brief explicitly forbids.
Adding runtime equality checks would retain a redundant invalid state in the
public contract, and adding another optional field or Codex side channel is out
of scope.

The contract needs a focused redesign with one authoritative source identity.
The smallest shape is for plugin Apply to use the source already carried by the
caller-resolved checkout (`checkout.source()`), while the exact
`plugin@marketplace` request identifies the selected marketplace name. If the
marketplace resource identity itself must cross the port, it should be part of
one resolved marketplace-checkout value rather than a second independent
`Source`.

Dispatch: direct-read only, as required by the caller. No production or test
files changed, no error contract was touched, and no assertions were weakened.
The story returned to `stage:drafting` so its Apply input can be made
unambiguous before becoming a public cross-crate contract.

## Implementation completion notes

- Execution capability: highest; this is a public cross-crate contract that
  controls source provenance and lifecycle evidence for every managed fallback
  adapter.
- Review weight: standard (project/autopilot run default).
- Dispatch: direct-read only, as required by the caller; no agent or peer
  delegation.
- Files changed: `crates/core/src/managed_projection.rs`,
  `crates/harnesses/src/managed_projection.rs`, and
  `crates/harnesses/src/lib.rs`.
- Tests updated: the harness contract test now exercises the object-safe
  `plan` method with `Apply` and `Remove`, checks all checkout accessors, and
  round-trips the complete manifest and aggregate fingerprints.
- Simplification: removed `AcquiredProjection`, `OmittedComponent`,
  `ManagedAcquisitionContext`, the parallel `omitted` field, and the split
  `acquire`/`project` methods. Apply now has exactly one authoritative source
  through `ResolvedSourceCheckout`; Remove has no source-bearing state.
- Discrepancy from design: `crates/core/src/lib.rs` already publicly exports
  the whole `managed_projection` module and had no named re-export list to
  update. The harness named re-export list was updated as designed.
- Invalid-state check: no second source identity or independently supplied
  marketplace source remains in the amended contract. No further design bounce
  was required.
- Verification: 332 core library tests and 26 harness library tests passed;
  `cargo check --workspace`, focused clippy with warnings denied, formatting,
  `git diff --check`, removed-symbol greps, duplicate-source greps, and the
  no-Codex-override check all passed.
- Adjacent issues parked: none.

## Acceptance criteria

- [x] `crates/core/src/managed_projection.rs` defines `ResolvedSourceCheckout`
      with `new`, `root`, `source`, `revision`, and `ManagedProjectionPlan`
      carries `manifest: Vec<ManagedProjection>`, `current_fingerprint:
      Option<Fingerprint>`, and `desired_fingerprint: Option<Fingerprint>`.
- [x] `OmittedComponent` and `AcquiredProjection` no longer exist in
      `crates/core/src/managed_projection.rs`; `ManagedProjectionPlan::omitted`
      is gone (omissions live in `manifest` as `ManagedProjection::Omitted`).
- [x] `crates/harnesses/src/managed_projection.rs` defines
      `ManagedProjectionInput<'a>` (`Apply { checkout }` | `Remove`) and one
      `ManagedProjectionContext` carrying
      `input: ManagedProjectionInput<'a>`; `ManagedAcquisitionContext` no
      longer exists.
- [x] `ManagedProjectionPort` exposes a single `plan` method taking
      `&ManagedProjectionContext<'_>` and returning
      `Result<ManagedProjectionPlan, ManagedProjectionError>`. `acquire` and
      `project` are gone.
- [x] `HarnessAdapter::managed_projection() -> Option<&dyn
      ManagedProjectionPort>` is unchanged; `CodexAdapter` still does not
      override it.
- [x] An updated interface test constructs a throwaway `ManagedProjectionPort`
      impl, invokes `plan` with `Apply` (round-tripping a
      `ResolvedSourceCheckout` plus a plan whose manifest/fingerprints equal
      the inputs) and with `Remove` (asserting no checkout is required),
      proving object safety and type round-trip for both input variants.
- [x] The approved `ManagedProjectionError` table test and the
      `contextual_summaries_vary_without_changing_the_typed_code` regression
      test pass unchanged (the error model is untouched).
- [x] `git grep -n "OmittedComponent\|AcquiredProjection\|ManagedAcquisitionContext" crates/`
      returns no matches (the removed names are gone from the public surface).
- [x] `cargo test -p skilltap-core --lib` and `cargo test -p
      skilltap-harnesses --lib` pass; `cargo clippy -p skilltap-core -p
      skilltap-harnesses --all-targets -- -D warnings`,
      `cargo fmt --all -- --check`, and `git diff --check` pass.
- [x] `cargo check --workspace` passes (downstream crates compile against the
      amended types).

## Out of scope

- Codex relocation onto the amended port (`feature-managed-fallback-target-
  parity-codex-adapter`, which consumes this amendment).
- The target-agnostic orchestrator, the checkout resolver, and the CLI
  dispatch flip (`feature-managed-fallback-target-parity-orchestrator`).
- The shared acceptance matrix (`feature-managed-fallback-target-parity-
  acceptance`).
- Any change to `ManagedProjectionError` codes, summaries, or the `Other`
  discipline — the approved error model is preserved verbatim.
- Any change to `ManagedProjection` / `PendingManagedAttempt` /
  `TargetResourceState` state shape or `STATE_SCHEMA_VERSION`.
- Claude managed-project lifecycle changes.
