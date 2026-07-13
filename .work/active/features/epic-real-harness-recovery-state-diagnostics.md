---
id: epic-real-harness-recovery-state-diagnostics
kind: feature
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on:
  - epic-real-harness-recovery-native-lifecycle
  - epic-real-harness-recovery-filesystem-instructions
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make lifecycle state and diagnostics target-exact

## Brief

Correct update summaries, help contracts, next-action aggregation, sequential
target widening, and dual-native state so agents receive one precise and
actionable account of every target. A logical plugin published natively to both
harnesses must keep separate target bindings and lifecycle evidence, never a
managed copy, and narrowed operations must preserve the sibling target.

This feature owns blocker inventory entries 12 and 16-20 plus the generic
post-mutation diagnostic friction that remains after native adapter repair. It
consumes the final lifecycle and instruction result contracts rather than
papering over their failures in rendering.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: final integration and agent-facing correctness feature.

## Foundation references

- `docs/SPEC.md` — state, plugin lifecycle, output, and exit codes.
- `docs/ARCH.md` — domain model, planning, updates, and error model.
- `docs/VISION.md` — native-first ownership and agent-readable operation.

## Design decisions

- **Persisted lifecycle evidence:** replace resource-wide lifecycle facts with
  a harness-keyed target binding map. A sidecar override map would create two
  sources of truth, while one state record per target would fragment the
  logical resource and duplicate its apply journal.
- **Inventory widening:** an install of an existing identical resource unions
  the selected targets into the desired resource. Conflicting source,
  component, or policy definitions still fail fast instead of being merged.
- **Update summary meaning:** `available_updates` counts resolved, changed,
  actionable candidates (`safe` or `needs_decision`) only. Blocked,
  unresolved, disabled, and unchanged candidates remain visible by status but
  do not inflate the count.
- **Plugin removal selector:** the public contract is the exact
  `<plugin>@<marketplace>` selector already required by lifecycle identity.
  Help and foundation examples must say so explicitly.
- **Next-action identity:** preserve first-seen order and suppress only exact
  duplicate actions (code, summary, and optional command). Similar actions
  with materially different commands or explanations remain visible.
- **Dispatch rationale:** direct-read design was sufficient because the four
  affected seams and their regression tests are localized and already named
  by the blocker inventory; exploratory fanout would duplicate the active
  implementation agents' work.

## Architectural choice

Model every successfully observed or applied harness representation as a
validated `TargetResourceState` stored beneath its logical `ResourceState`.
The logical record retains only the scope-bearing resource key; all facts that
can differ by harness—native identity, source,
revision, ownership, provenance, artifact, fingerprint, and observation
time, including the apply journal—live in the target binding. This is the
single-source-of-truth option.

Rejected alternatives were (1) retaining the current resource-wide fields and
adding per-target overrides, which makes every consumer implement precedence,
and (2) storing one `ResourceState` per harness, which breaks logical resource
identity and makes narrowed updates/removals reconstruct sibling state. The
target binding map makes target projection a typed operation and lets native
and managed representations coexist without pretending the whole resource has
one ownership class.

The trickiest unit is the target-evidence wire contract. It must reject a map
key that disagrees with its binding, invalid provenance/ownership/artifact
combinations, and accidental deletion of an unselected sibling while allowing
the lifecycle pipeline to build evidence incrementally. The implementation
must update the strict state golden and every state mutator together; it must
not leave compatibility aliases for the resource-wide shape.

## Implementation units

### Unit 1: Target-bound lifecycle state

**Files:**

- `crates/core/src/storage/state.rs`
- `crates/core/src/storage/mod.rs`
- `crates/core/src/storage/tests.rs`
- `crates/core/src/storage/fixtures/state.json`
- `crates/core/src/publication.rs`
- `crates/core/src/foreground_update.rs`
- `crates/core/tests/storage_integration.rs`

**Story:** `epic-real-harness-recovery-state-diagnostics-target-evidence`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "TargetResourceStateWire")]
pub struct TargetResourceState {
    harness: HarnessId,
    native_id: Option<NativeId>,
    provenance: Provenance,
    ownership: Ownership,
    source: Option<Source>,
    managed_artifact: Option<ManagedArtifactRecord>,
    fingerprint: Option<Fingerprint>,
    installed_revision: Option<ResolvedRevision>,
    available_revision: Option<ResolvedRevision>,
    observed_at: Timestamp,
    last_apply: Option<ApplyRecord>,
}

impl TargetResourceState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        harness: HarnessId,
        native_id: Option<NativeId>,
        provenance: Provenance,
        ownership: Ownership,
        source: Option<Source>,
        managed_artifact: Option<ManagedArtifactRecord>,
        fingerprint: Option<Fingerprint>,
        installed_revision: Option<ResolvedRevision>,
        available_revision: Option<ResolvedRevision>,
        observed_at: Timestamp,
        last_apply: Option<ApplyRecord>,
    ) -> Result<Self, SchemaError>;
}

pub struct ResourceState {
    key: ResourceKey,
    targets: BTreeMap<HarnessId, TargetResourceState>,
}

impl ResourceState {
    pub fn new(
        key: ResourceKey,
        targets: impl IntoIterator<Item = TargetResourceState>,
    ) -> Result<Self, SchemaError>;

    pub fn targets(&self) -> &BTreeMap<HarnessId, TargetResourceState>;
    pub fn with_target(&self, target: TargetResourceState) -> Result<Self, SchemaError>;
    pub fn without_targets(&self, selected: &HarnessSet) -> Result<Option<Self>, SchemaError>;
}
```

**Implementation notes:**

- Serialize through private `deny_unknown_fields` wire DTOs and rebuild via
  validating constructors, following `validated-wire-contract`.
- A target binding owns every lifecycle field that can legitimately differ by
  harness. Its `harness` must equal its map key. Artifact role and
  provenance/ownership validation remains fail-fast per binding.
- Persist `ApplyRecord` in the target binding so narrowed removal drops exactly
  that target's journal and a later reinstall cannot inherit a sibling's
  `previously_applied` claim.
- The state schema changes atomically. Update the schema constant and strict
  golden in the same story; unsupported older shapes fail with the existing
  schema-version diagnostic rather than being guessed or partially decoded.
- Publication and verified update recording mutate only the selected target
  bindings. They never copy one harness's native revision or provenance into a
  sibling binding.

**Acceptance criteria:**

- [ ] One logical resource can represent Codex-native and Claude-native
      bindings with distinct native IDs, source identities, installed and
      available revisions, provenance, ownership, and timestamps.
- [ ] A native binding and a skilltap-materialized sibling binding validate
      without a resource-wide ownership lie.
- [ ] Removing or updating one binding preserves the sibling byte-for-byte and
      cannot leave stale selected-target apply evidence.
- [ ] Malformed, duplicate, mismatched-key, and invalid ownership/artifact
      bindings fail deserialization.
- [ ] State round trips through the strict golden and no consumer reads a
      resource-wide lifecycle field.

### Unit 2: Target-union inventory and dual-native lifecycle

**Files:**

- `crates/core/src/storage/inventory.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application.rs`
- `crates/cli/tests/compiled_binary.rs`
- `crates/test-support/src/lib.rs`

**Story:** `epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle`

```rust
impl InventoryDocument {
    pub fn upsert_resource_targets(
        &self,
        proposed: DesiredResource,
    ) -> Result<Self, SchemaError>;
}

fn merge_identical_resource_targets(
    existing: &DesiredResource,
    proposed: &DesiredResource,
) -> Result<DesiredResource, ResourceContractError>;

fn project_state_targets_after_remove(
    repository: &dyn StateRepository,
    keys: &BTreeSet<ResourceKey>,
    selected: &HarnessSet,
) -> Result<(), LifecycleStateError>;
```

**Implementation notes:**

- `upsert_resource_targets` unions targets only after all non-target desired
  fields are equal. It preserves existing accepted consequences for sibling
  targets and takes proposed consequences only for newly selected targets.
- Replace the current branch that reuses an existing same-source resource
  unchanged. A second narrowed install widens inventory before planning and
  creates operations only for the missing target.
- Build state evidence from each target's verified native result. Never seed a
  sibling from the requested selector or reuse a resource-wide revision.
- Removal projects both desired inventory and target evidence; the resource is
  deleted only when the last desired/bound target is removed.
- Extend isolated fake native fixtures only to reproduce documented native
  command results; real binaries remain covered by the parent epic's
  disposable-home pass.

**Acceptance criteria:**

- [ ] `install --target codex` followed by the same install for Claude widens
      desired targets to both and mutates only Claude on the second command.
- [ ] Repeating either narrowed install is a no-op.
- [ ] `--target all` install/update/remove executes exact native lifecycle for
      both harnesses and records distinct target evidence.
- [ ] A narrowed update or removal preserves the sibling desired target,
      native installation, revision, provenance, ownership, and journal.
- [ ] A dual-native plugin never creates a managed plugin artifact.
- [ ] Conflicting definitions with the same resource key fail without
      widening or mutation.

### Unit 3: Actionable update availability

**Files:**

- `crates/core/src/updates.rs`
- `crates/cli/src/application/status.rs`
- `crates/cli/tests/compiled_binary.rs`

**Story:** `epic-real-harness-recovery-state-diagnostics-update-eligibility`

```rust
impl UpdateDecision {
    pub const fn is_actionable_update(self) -> bool;
}

fn status_update_projection(
    documents: &StatusDocuments,
    scope: &StatusScope,
    targets: &StatusTargets,
    observation: &NativeObservation,
) -> (Vec<OutputEntry>, Vec<Warning>, usize);
```

**Implementation notes:**

- Centralize the summary predicate on the domain decision: only `Safe` and
  `NeedsDecision` are actionable updates. `Blocked` remains a visible status
  with a reason but is not an available update.
- Resolution failures, local instruction entries, and non-resolvable local
  path skills must remain warnings/status entries without incrementing the
  summary.
- Read installed and available revisions from the exact selected target
  binding. When selected targets disagree, emit target-specific update entries
  rather than collapsing them into one revision.

**Acceptance criteria:**

- [ ] Unresolved, blocked, disabled, and unchanged candidates contribute zero
      to `available_updates`.
- [ ] A resolved changed safe candidate and a resolved changed
      decision-required candidate each contribute one per exact target entry.
- [ ] Local instructions and local-path skills do not appear as phantom
      available updates.
- [ ] Plain and JSON status expose the same count and target-specific reasons.

### Unit 4: Exact help and stable diagnostic aggregation

**Files:**

- `crates/cli/src/command.rs`
- `crates/cli/src/outcome.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application/reconciliation.rs`
- `crates/cli/src/output.rs`
- `crates/cli/src/command/tests.rs`
- `crates/cli/src/output/tests.rs`
- `crates/cli/tests/compiled_binary.rs`
- `docs/SPEC.md`
- `docs/UX.md`

**Story:** `epic-real-harness-recovery-state-diagnostics-output-contract`

```rust
impl Outcome {
    pub fn with_next_action(self, action: NextAction) -> Self;
    pub fn extend_next_actions(
        self,
        actions: impl IntoIterator<Item = NextAction>,
    ) -> Self;
    pub fn normalize_next_actions(&mut self);
}

pub struct PluginRemoveArgs {
    #[arg(
        value_name = "PLUGIN@MARKETPLACE",
        value_parser = parse_plugin_selector
    )]
    pub plugin: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}
```

**Implementation notes:**

- Give plugin removal its own argument type so marketplace and plugin help
  cannot drift while sharing `PluginNamedArgs`. Update SPEC and UX command
  examples in place to the exact selector.
- Make ordered deduplication an `Outcome` invariant at construction and merge
  boundaries. Replace direct `Vec::extend` aggregation in lifecycle and
  reconciliation with the invariant-preserving method; normalize once before
  rendering as a defense for legacy internal call sites.
- Exact equality includes code, summary, and command. Preserve first-seen order
  so plain and JSON rendering remain stable.
- Consume typed post-mutation errors and recovery actions from the repaired
  native lifecycle feature. Do not replace them with a generic observation
  message or invent a successful result when final observation is unhealthy.

**Acceptance criteria:**

- [ ] Root, group, and leaf help state that plugin removal requires
      `PLUGIN@MARKETPLACE`; malformed bare names fail at parsing with the same
      contract in plain and JSON entry paths.
- [ ] Identical next actions from multiple targets/scopes render once in
      first-seen order in plain and JSON.
- [ ] Actions that differ by command or explanation are retained.
- [ ] A native post-mutation failure reports the repaired adapter's precise
      boundary reason and one actionable recovery command; a healthy final
      observation completes normally.
- [ ] Outcome normalization is idempotent and never changes result class or
      exit code.

## Implementation order

1. `epic-real-harness-recovery-state-diagnostics-target-evidence` establishes
   the target-scoped storage and mutation contract.
2. `epic-real-harness-recovery-state-diagnostics-update-eligibility` can run
   in parallel because its classification predicate is independent of the
   wire rewrite; its target projection integration is verified again after
   Unit 1 lands.
3. `epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle`
   consumes target evidence and repairs sequential widening plus narrowed
   preservation.
4. `epic-real-harness-recovery-state-diagnostics-output-contract` consumes the
   final lifecycle/update outcomes and closes agent-facing help and diagnostic
   aggregation.

## Testing

### Unit tests

- `crates/core/src/storage/tests.rs` and the strict state golden cover target
  binding validation, round trips, projection, distinct evidence, and schema
  rejection.
- `crates/core/src/updates.rs` covers the actionable-summary truth table.
- `crates/core/src/storage/inventory.rs` covers exact target union and
  non-target conflict rejection.
- `crates/cli/src/outcome.rs` and `crates/cli/src/output/tests.rs` cover stable
  ordered deduplication and plain/JSON parity.
- `crates/cli/src/command/tests.rs` covers exact plugin removal grammar and
  generated help.

### Integration tests

- `crates/cli/tests/compiled_binary.rs` runs dual-native install, sequential
  sibling widening, update, narrowed preservation, removal, and immediate
  repeat no-ops in isolated homes with exact per-target state assertions and
  no managed plugin artifact.
- The same suite covers blocked/unresolved/local update summaries, duplicate
  aggregation from multi-target/all-scope commands, and precise typed
  post-mutation diagnostics.
- The parent epic's final real-Codex/real-Claude pass repeats the dual-native
  scenarios in disposable roots only; no test reads or writes the operator's
  harness configuration.

## Risks

- **Riskiest assumption:** every currently resource-wide lifecycle fact can be
  assigned to one target without losing a required shared invariant. The
  fallback is to keep only the logical resource key shared; do not reintroduce
  field-level fallback precedence.
- **Schema risk:** a partial wire conversion could deserialize plausible but
  ambiguous state. The strict DTO, schema bump, golden, and constructor-only
  decoding make the change atomic and fail closed.
- **Lifecycle risk:** incremental target widening could publish desired state
  before the new target mutation fails. Existing recovery semantics retain the
  desired target and report the exact retry; they must not erase the already
  healthy sibling.
- **Counting risk:** hiding blocked candidates from `available_updates` could
  hide their diagnostic entirely. They remain explicit update entries and
  warnings; only the headline count changes.
- **Aggregation risk:** deduplicating by code alone could remove materially
  distinct instructions. Exact value equality and first-seen order avoid that
  loss.

## Children complete (2026-07-12)

All direct stories are terminal; the feature advanced through review.

## Review (2026-07-12, bounded final pass)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep aggregate review. Per-target state, actionable
update counts, dual-native widening, exact output normalization, and typed
post-mutation diagnostics satisfy the feature contract. Focused output/native
tests, the full workspace suite, and strict Clippy pass.
