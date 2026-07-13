---
id: epic-real-harness-recovery-native-lifecycle
kind: feature
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: [epic-real-harness-recovery-runtime-boundary]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align native lifecycle adapters with current harness contracts

## Brief

Correct Codex and Claude marketplace/plugin observation and mutation vectors,
the attested capability-profile registry, project-scope capability behavior,
update fallbacks, and post-mutation observation. Commands must match current
real CLI help and isolated execution before current versions gain mutation
authority; absent native lifecycle must select the documented managed load-path
fallback or remain explicitly unsupported rather than invoke an invented
command.

This feature owns blocker inventory entries 2, 5-7, and 9-10. It consumes the
runtime feature's version and root model. It does not redesign stored dual-target
provenance or general output aggregation, which are handled by the
state/diagnostics feature.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: consumes the repaired runtime boundary and enables real
  native marketplace/plugin verification.

## Foundation references

- `docs/HARNESS-CONTRACTS.md` — Codex and Claude native commands, roots, and
  materialization fallback.
- `docs/ARCH.md` — harness adapter contract and plugin resolution.

## Design decisions

- **Authority registry:** replace the synthetic shared `3.0.0` switch with an
  immutable per-harness, exact-version registry. Codex `0.144.1` and Claude
  Code `2.1.201` receive only the operation/scope combinations verified by
  current help and disposable-home execution; unknown versions remain
  observe-only and runtime probes may only narrow support.
- **Operation-specific Claude scope encoding:** plugin mutations and the
  marketplace mutations that accept `--scope` retain `user`/`local`; plugin
  list, marketplace list, and marketplace update omit the rejected flag.
  Project observations and unscoped mutations run with the project root as
  their working directory and must confirm the reported resource scope before
  state is accepted.
- **Codex plugin update:** Codex `0.144.1` reports native plugin update as
  unsupported. Do not invent `codex plugin update` or silently compose a
  remove/install replacement; a future exact profile may authorize an
  attested replacement strategy.
- **Codex project fallback:** an unsupported native project operation resolves
  to skilltap-owned materialization only when the explicit source can be read
  and the result can be represented through documented project marketplace,
  plugin, skill, and MCP load paths. Native caches stay read-only; missing or
  partial required components remain blocked under the normal acknowledgment
  contract.
- **Observation is a postcondition:** list-command failure, invalid JSON/shape,
  ambiguous scope, and an unmet expected presence are distinct adapter
  outcomes. A mutation is not journaled as successful until the target-specific
  postcondition is freshly observed; indeterminate evidence never becomes a
  repeat no-op.
- **Dispatch rationale:** direct-read design was sufficient after mapping the
  profile registry, vector builder, lifecycle application service,
  publication core, and compiled-binary fixtures. The implementation is split
  at authority, managed fallback, and postcondition seams so the first and
  second stories can proceed independently after the runtime dependency.
- **Foundation timing:** code-first for `docs/HARNESS-CONTRACTS.md`. Its Claude
  scope claims must be corrected in the contract/vector story alongside the
  implementation and real-command evidence.

## Architectural choice

Use one typed lifecycle resolver that chooses among a verified native command,
a documented managed projection, and an explicit unavailable result. The
resolver consumes the exact compiled profile and concrete scope; executors do
not reinterpret an unverified capability. Native and managed paths then share
one postcondition protocol before state/journal publication.

This keeps mutation authority explicit and preserves Ports & Adapters: profile
selection and command construction stay in `skilltap-harnesses`, normalized
managed publication stays in core, and `skilltap-cli` composes them without
embedding harness command folklore. It also makes the documented preference
order executable rather than treating `Unverified` as the end of the request.

Two alternatives were rejected. Patching individual argument arrays while
retaining the fictitious shared profile would make today's tests pass but
would still authorize unverified commands. Treating every missing native
operation as a generic materialization would erase the important distinction
between an attested load path and an unsupported component graph.

The trickiest unit is Codex project fallback because it must reuse the existing
bounded source/graph and managed-publication machinery without writing a native
cache or claiming native provenance. Its plan is constructed before mutation,
names every destination, carries compatibility consequences, and is accepted
only after load-path observation confirms the projected plugin.

## Implementation units

### Unit 1: Exact profiles and operation-specific native vectors

**Story:** `epic-real-harness-recovery-native-lifecycle-contracts`

**Files:**

- `crates/harnesses/src/lib.rs`
- `crates/harnesses/src/lifecycle.rs`
- `crates/harnesses/tests/detection.rs`
- `crates/harnesses/tests/lifecycle.rs`
- `crates/test-support/src/native_process.rs` or the current fake-native module
- `crates/cli/tests/compiled_binary.rs`
- `docs/HARNESS-CONTRACTS.md`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeLifecycleStrategy {
    Direct(NativeCommandContract),
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCommandContract {
    pub action: NativeLifecycleAction,
    pub scope_encoding: NativeScopeEncoding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeScopeEncoding {
    Flag,
    WorkingDirectory,
    GlobalOnly,
}

pub fn select_profile(
    harness: HarnessKind,
    version: &NativeVersion,
) -> CapabilityProfileSelection;

pub fn native_arguments(
    request: &NativeLifecycleRequest,
    contract: NativeCommandContract,
) -> Result<Vec<OsString>, NativeLifecycleError>;

pub fn native_list_arguments(
    request: &NativeLifecycleRequest,
) -> Result<Vec<OsString>, NativeLifecycleError>;
```

**Implementation notes:**

- Keep one static registry keyed by `(HarnessKind, exact NativeVersion)` and
  derive profile IDs/capability sets from it. Do not accept ranges, aliases,
  or a version parsed from a different harness.
- Codex `0.144.1` authorizes global marketplace add/remove/upgrade and plugin
  add/remove, but not plugin update. Project native lifecycle remains
  unsupported and is resolved by Unit 2.
- Claude `2.1.201` uses unscoped JSON list vectors. Marketplace update also
  omits `--scope`; project execution uses the exact project working directory
  and later validates reported scope. Preserve `--scope user|local` only on
  operations whose current help and real execution accept it.
- Fake binaries must emulate the real grammar and reject forbidden flags or
  commands so synthetic tests cannot reintroduce blockers 5-7.

**Acceptance:**

- [ ] Current real Codex and Claude versions select exact verified profiles;
      synthetic `3.0.0`, adjacent versions, and unknown versions cannot mutate.
- [ ] Claude plugin/marketplace lists and marketplace update contain no
      rejected `--scope`; scoped mutations retain only supported flags.
- [ ] Codex never emits a plugin-update command for `0.144.1` and reports the
      capability unavailable with an actionable result.
- [ ] Global/project working directories and explicit process environments
      remain the exact values supplied by the runtime boundary.
- [ ] Contract docs and fake fixtures match the real isolated CLI grammar.

### Unit 2: Managed Codex project lifecycle fallback

**Story:** `epic-real-harness-recovery-native-lifecycle-managed-project`

**Files:**

- `crates/cli/src/application/lifecycle.rs`
- `crates/harnesses/src/lifecycle.rs`
- `crates/harnesses/src/plugin_graph.rs`
- `crates/core/src/publication.rs`
- `crates/core/src/storage/managed_artifact.rs`
- `crates/cli/tests/compiled_binary.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedLifecycle {
    Native(NativeLifecycleRequest),
    Managed(ManagedLifecycleRequest),
    Unavailable(LifecycleUnavailable),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedLifecycleRequest {
    pub resource: ResourceKey,
    pub target: HarnessId,
    pub scope: Scope,
    pub source: Source,
    pub action: OperationAction,
}

pub fn resolve_lifecycle(
    request: &NativeLifecycleRequest,
    support: CapabilitySupport,
    source: Option<&Source>,
) -> ResolvedLifecycle;
```

**Implementation notes:**

- Resolve unsupported Codex project marketplace/plugin operations before
  building an execution port. A documented managed route is a positive
  capability, not a coercion of `Unverified` into native support.
- Marketplace registration edits the documented project marketplace file
  through a validating, unknown-field-preserving adapter. Plugin install/update
  reads the explicit source, validates the complete graph, and plans owned
  project load-path artifacts through the existing publication batch.
- Removal requires matching skilltap ownership and current fingerprint; drift
  or an unmanaged destination blocks deletion. State records
  `Provenance::Materialized` and `Ownership::Skilltap`, never native.
- Compatibility still decides whether the projection is faithful, partial, or
  blocked. `--yes` may acknowledge disclosed optional loss but cannot waive a
  missing required skill/MCP dependency.

**Acceptance:**

- [ ] Codex project marketplace and plugin operations use documented managed
      paths when native project lifecycle is unavailable, without invoking a
      Codex mutation command or writing a cache.
- [ ] Complete skills and MCP configuration are projected with correct
      global/project ownership and component compatibility evidence.
- [ ] Unsupported required components block; optional loss requires the normal
      plan acknowledgment; faithful projections need no acknowledgment.
- [ ] Update/remove reject drift and unmanaged destinations, preserve unknown
      marketplace fields, and repeat successful operations as no-ops.
- [ ] Global Codex and every Claude native path remain native when their exact
      profiles authorize them.

### Unit 3: Typed observation and verified lifecycle postconditions

**Story:** `epic-real-harness-recovery-native-lifecycle-postconditions`

**Files:**

- `crates/harnesses/src/lifecycle.rs`
- `crates/cli/src/application.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/core/src/executor.rs`
- `crates/cli/tests/compiled_binary.rs`
- isolated real-harness validation scripts/fixtures already used by the epic

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeResourceObservation {
    Present { scope: Option<CapabilityScope> },
    Missing,
    Indeterminate(NativeObservationFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeObservationFailure {
    CommandFailed,
    InvalidJson,
    UnsupportedShape,
    AmbiguousScope,
}

pub fn observe_native_resource(
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    environment: &BTreeMap<OsString, OsString>,
    request: &NativeLifecycleRequest,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<NativeResourceObservation, NativeLifecycleError>;

pub fn verify_lifecycle_postcondition(
    action: NativeLifecycleAction,
    requested_scope: &Scope,
    observation: &NativeResourceObservation,
) -> Result<(), LifecyclePostconditionError>;
```

**Implementation notes:**

- Parse only bounded documented list shapes, but retain normalized entry scope
  when Claude returns it. Raw payloads and stderr never cross the adapter.
- Distinguish preflight indeterminacy from missing. A prior journal entry plus
  indeterminate fresh evidence is attention-required, not `no_change`.
- After every successful native mutation, re-observe through the same bound
  executable/environment. Install/add/update require present in the requested
  scope; remove requires missing in that scope.
- Map failures to stable stage-specific codes such as
  `native.observation_command_failed`, `native.observation_contract_changed`,
  and `native.postcondition_not_met`. Do not publish a successful journal/state
  seed when verification fails; preserve enough operation evidence for a safe
  retry and next action.

**Acceptance:**

- [ ] Successful native mutations are recorded only after their exact
      target/scope postcondition is observed.
- [ ] Nonzero list, malformed JSON, changed shapes, ambiguous scope, and unmet
      presence each produce actionable, stable diagnostics rather than a
      generic observation failure.
- [ ] Indeterminate preflight evidence never suppresses a needed operation or
      turns into a false repeat no-op.
- [ ] Native and managed lifecycle repeats are zero-change only after fresh
      target-specific evidence agrees with stored ownership/provenance.
- [ ] Real isolated Claude/Codex tests cover successful mutation, failure,
      post-observation failure, and repeat behavior without touching the user
      environment.

## Implementation order

1. `epic-real-harness-recovery-native-lifecycle-contracts`
2. `epic-real-harness-recovery-native-lifecycle-managed-project` (may proceed
   in parallel with Unit 1 once the runtime boundary is stable)
3. `epic-real-harness-recovery-native-lifecycle-postconditions`

## Testing

### Unit and adapter tests

- Table-test every exact version/profile/scope/action combination, including
  unknown and adjacent versions.
- Golden-test all Codex and Claude argument vectors against the current real
  help grammar; forbidden flags and nonexistent commands are negative cases.
- Exercise bounded list parsing for arrays, wrapped lists, malformed entries,
  explicit Claude scopes, wrong scopes, nonzero exits, and size/depth limits.

### Application and isolated integration tests

- Use test-support-owned homes, config/cache roots, project Git roots, and fake
  binaries that reject the same invalid vectors as current real CLIs.
- Cover native global Claude/Codex lifecycle, Claude project lifecycle, Codex
  project managed fallback, partial/blocking compatibility, drift-safe remove,
  and immediate repeat no-ops.
- Run the installed Codex `0.144.1` and Claude `2.1.201` binaries only inside
  disposable roots and confirm commands, filesystem destinations, provenance,
  and postconditions. Never read or mutate the operator's native roots.

## Risks

- **Riskiest assumption:** unscoped Claude marketplace update uses the current
  working directory to select local state. The implementation must prove that
  in a disposable project; if scope cannot be observed unambiguously, leave
  project update unverified instead of guessing.
- **Fallback ownership:** a documented load path may contain user-authored
  content. Publication/removal must fail closed unless provenance and current
  fingerprints prove skilltap ownership.
- **Partial native success:** a command can mutate successfully and then become
  unobservable. Preserve the attention result without claiming success or
  automatically undoing a change whose final native state is unknown.

## Children complete (2026-07-12)

All direct stories are terminal; the feature advanced through review.

## Review (2026-07-12, bounded final pass)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep aggregate review. Native vectors, scope-aware
presence, managed Codex project projections, postcondition retry safety, and
exact journal recovery are complete and mutually consistent. Full workspace
tests and strict Clippy pass in isolated roots.
