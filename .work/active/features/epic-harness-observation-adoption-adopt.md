---
id: epic-harness-observation-adoption-adopt
kind: feature
stage: implementing
tags: []
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-status]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Conflict-Aware Adoption

Implement pure adoption candidate, coalescing, equivalence, conflict, and
idempotence contracts over the shared fresh snapshot. Under the configuration
lock, reload inventory, revalidate selected identities/fingerprints, preserve
manual/unrelated entries, add every non-conflicting candidate with adopted
source provenance, and publish one atomic inventory replacement. Single-source
resources target their source harness; already equivalent multi-harness
resources may coalesce. Shared Claude project declarations remain unadoptable.
Adoption never calls a native mutation, writes observation state, transfers a
resource, or discards healthy siblings because another conflicts.

## Architectural choice

Adoption uses a pure core planner followed by a small locked application
service. The planner receives one fresh, ephemeral normalized observation and
the current inventory and returns typed decisions, desired additions, and the
native identity/fingerprint evidence that must be revalidated before commit.
The application service acquires the existing process-wide configuration lock,
reloads inventory, re-observes only the selected candidates, recomputes the
plan against that locked inventory, and publishes one validated atomic
replacement. The CLI only resolves scope/target, composes the adapters and
repositories, and renders the typed result.

Merging directly in the CLI would duplicate policy and make conflict behavior
hard to exercise. Persisting the observation snapshot would violate the
ephemeral observation contract and could make a later adoption stale. The
planner-plus-locked-commit split preserves the ports-and-adapters boundary,
keeps adoption read-only toward native state, and makes a repeated invocation
an inventory no-op.

## Design decisions

- **Candidate layer**: use effective observations as the installed resource
  candidate; a declared observation may supply lineage only when the adapter
  explicitly correlates it to that effective instance. A declared-only,
  malformed, unresolved, or unsupported instance remains an attention finding,
  not an invented desired resource.
- **Identity and equivalence**: a candidate is keyed by its exact
  `ResourceKey` (logical id plus concrete scope). Cross-harness coalescing also
  requires the same declared `Source`, `ResourceKind`, and complete
  `ComponentGraph`; names, URLs, copied bytes, caches, and revision equality
  alone never prove equivalence. Fingerprints are revalidation evidence, not
  identity.
- **Provenance**: every new entry uses `DesiredOrigin::Adopted(source_harness)`
  and targets its source harness. A coalesced candidate targets every
  equivalent selected source harness and records the lexicographically first
  source harness in the single-value origin field; the decision retains all
  contributing harnesses for output. Existing inventory entries and their
  explicit policy fields are never rewritten by adoption.
- **Conflicts and shared scope**: an existing key with different semantics,
  an ambiguous source, or a Claude project declaration marked
  `Adoptable(false)` is reported as a conflict/unadoptable decision. It is
  skipped while unrelated candidates continue. Shared Claude project
  declarations remain health evidence only.
- **Commit safety**: the lock is fail-fast. After lock acquisition inventory
  is loaded again, selected native identities and fingerprints are revalidated,
  and the pure planner is rerun. If any selected evidence changed, the whole
  adoption commit is rejected with an actionable stale-observation result;
  healthy unrelated inventory entries are preserved. An empty merge does not
  create or rewrite `inventory.toml`.
- **Output**: `adopt` has no acknowledgment flag. Plain and `--json` output
  render the same typed decisions (`adopted`, `coalesced`, `already_managed`,
  `conflict`, `unadoptable`, and `unchanged`) and return attention when a
  conflict, skipped shared declaration, partial sibling, or stale evidence
  requires a user decision.

## Tricky unit

The highest-risk unit is effective-candidate coalescing. Native observations
contain declared/effective layers, per-harness failures, and scope-bearing
keys; a name or equal fingerprint must not accidentally turn two resources
into one. The planner therefore groups only exact keys, validates source and
component semantics before combining harnesses, and carries each candidate's
identity/fingerprint envelope into the locked revalidation step.

## Implementation units

### Unit 1: Pure adoption candidates and decisions

**File**: `crates/core/src/adoption.rs` (new; exported from `crates/core/src/lib.rs`)

```rust
pub struct AdoptionSelection {
    pub targets: BTreeSet<ObservationTarget>,
}

pub struct AdoptionIdentity {
    pub target: ObservationTarget,
    pub observation: ObservationKey,
    pub native_identity: NativeId,
    pub fingerprint: Option<Fingerprint>,
}

pub struct AdoptionCandidate {
    pub desired: DesiredResource,
    pub identity: AdoptionIdentity,
    pub source_harnesses: HarnessSet,
}

pub enum AdoptionDecision {
    Adopted(AdoptionCandidate),
    Coalesced(AdoptionCandidate),
    AlreadyManaged { key: ResourceKey },
    Conflict { key: ResourceKey, code: AdoptionConflictCode },
    Unadoptable { key: ResourceKey, code: AdoptionUnadoptableCode },
    Unchanged { key: ResourceKey },
}

pub struct AdoptionPlan {
    pub decisions: Vec<AdoptionDecision>,
    pub additions: Vec<DesiredResource>,
    pub evidence: BTreeMap<AdoptionIdentity, ObservationEvidenceDigest>,
}

pub fn plan_adoption(
    inventory: Option<&InventoryDocument>,
    environment: &ObservedEnvironment,
    selection: &AdoptionSelection,
) -> Result<AdoptionPlan, AdoptionError>;
```

`plan_adoption` walks only selected concrete targets, consumes effective
resources, retains partial/failure findings, and returns deterministic sorted
decisions. Conversion to `DesiredResource` defaults to `UpdateIntent::Track`,
all observed components are `ComponentChoice::Default`, and observed resolved
dependencies become exact scope-bearing desired dependencies. Candidates with
invalid component/dependency contracts are unadoptable rather than repaired.
Existing inventory is consulted for exact-key idempotence and semantic
conflict, but its policy, targets, origin, and component choices are never
silently replaced.

### Unit 2: Equivalence, provenance, and inventory merge

**File**: `crates/core/src/adoption.rs` (same module; pure helpers and tests)

```rust
pub fn equivalent_candidates(
    left: &AdoptionCandidate,
    right: &AdoptionCandidate,
) -> bool;

pub fn merge_inventory(
    inventory: &InventoryDocument,
    additions: impl IntoIterator<Item = DesiredResource>,
) -> Result<InventoryDocument, AdoptionError>;
```

Equivalence requires equal `ResourceKey`, source, kind, component graph, and
dependencies. Fresh candidates with equal semantics can union their target
harnesses and choose a stable origin source; different semantics at one key
produce a conflict and no write. `merge_inventory` preserves every unrelated
resource and recorded project, adds project scopes to `projects`, and returns a
byte-identical logical document when no addition is new. It never compares or
merges by native cache location or fingerprint.

### Unit 3: Locked incremental inventory publication

**File**: `crates/core/src/adoption.rs` (application port) and
`crates/core/src/storage/repository.rs` (existing atomic inventory adapter)

```rust
pub fn apply_adoption<L, R, F>(
    lock: &L,
    lock_path: &AbsolutePath,
    inventory: &R,
    plan: &AdoptionPlan,
    reobserve: F,
) -> Result<AdoptionApplyResult, AdoptionApplyError>
where
    L: ConfigurationLock,
    R: InventoryRepository,
    F: FnOnce(&BTreeSet<AdoptionIdentity>)
        -> Result<ObservedEnvironment, AdoptionObservationError>;
```

The service fails fast on contention, acquires before the second inventory
load, re-observes only `plan.evidence` identities, rejects changed native
identity/fingerprint envelopes, reruns `plan_adoption` against the locked
inventory, validates the complete `InventoryDocument`, and calls the existing
atomic `InventoryRepository::replace` once. Missing inventory is treated as an
empty document only after the lock is held. No state snapshot, managed
artifact, native file, or harness command is written. Lock release is explicit
and RAII cleanup covers all error paths.

### Unit 4: Non-interactive adopt command

**Files**: `crates/cli/src/dispatch.rs`, `crates/cli/src/application.rs`,
`crates/cli/src/entrypoint.rs`, and `crates/cli/src/output.rs`.

```rust
pub(crate) struct AdoptApplication<'a> {
    pub(crate) config: &'a dyn ConfigRepository,
    pub(crate) inventory: &'a dyn InventoryRepository,
    pub(crate) scopes: &'a ScopeResolver<'a>,
    pub(crate) working_directory: &'a dyn WorkingDirectory,
    pub(crate) observation: &'a dyn AdoptObservationService,
    pub(crate) lock: &'a dyn ConfigurationLock,
    pub(crate) lock_path: &'a AbsolutePath,
}

impl AdoptApplication<'_> {
    pub(crate) fn execute(&self, args: &AdoptArgs) -> Outcome;
}
```

Dispatch routes `Command::Adopt`; entrypoint composition supplies native
Codex/Claude observers, the selected configuration lock, and file repositories.
`--from` resolves to one enabled harness or all enabled harnesses; omitted
`--from` means all enabled. Scope expansion is exact: global by default,
current/explicit project for `--project`, and global plus inventory-recorded
projects for `--all-scopes`. Disabled explicit targets and absent enabled
harnesses fail before observation. The command never searches marketplace
contents and never accepts `--yes`.

### Unit 5: Adoption contract and integration tests

**Files**: `crates/core/src/adoption/tests.rs`,
`crates/core/tests/storage_integration.rs`, `crates/cli/src/application/tests.rs`,
and `crates/cli/tests/compiled_binary.rs`.

Fixtures cover effective/declared pairs, equivalent Codex/Claude resources,
same-key semantic conflicts, source-less and shared Claude project resources,
unresolved dependencies, partial sibling failures, stale identity/fingerprint
revalidation, lock contention, manual inventory edits between observations,
global/current/explicit/all scopes, and repeated adoption. The compiled CLI
asserts that native trees, config, state, symlinks, bytes, types, and mtimes
remain unchanged; only `inventory.toml` is atomically created or replaced when
there is a new non-conflicting candidate.

## Implementation order

1. `epic-harness-observation-adoption-adopt-candidates` — candidate conversion,
   effective-layer filtering, typed decisions, and evidence envelopes.
2. `epic-harness-observation-adoption-adopt-merge` — conservative equivalence,
   stable provenance, and inventory merge semantics; depends on candidates.
3. `epic-harness-observation-adoption-adopt-persistence` — lock, reload,
   revalidation, and one atomic inventory publication; depends on merge.
4. `epic-harness-observation-adoption-adopt-cli` — exact scope/target command,
   adapter composition, and output; depends on persistence.
5. `epic-harness-observation-adoption-adopt-integration` — end-to-end safety,
   conflict, partial, and repeat-adoption contracts; depends on CLI.

## Testing

- Pure adoption tests assert deterministic candidate ordering, effective-layer
  selection, source/semantic equivalence, scope-bearing identity, conflict
  isolation, shared-project exclusion, and idempotent merges.
- Storage tests assert lock contention fails fast, inventory is reloaded after
  locking, changed evidence aborts publication, unrelated manual entries are
  preserved, and atomic replacement happens at most once.
- CLI tests assert exact target/scope expansion, stable plain/JSON output and
  exit classes, no marketplace discovery, no state/native writes, and immediate
  repeat adoption returns `unchanged` with no inventory rewrite.

## Risks

- Native adapters may not yet expose enough typed lineage to distinguish a
  declared/effective pair. The fallback is an explicit `unadoptable` finding,
  never a guessed resource.
- `DesiredOrigin` currently stores one source harness. Coalesced decisions keep
  all contributing harnesses in the ephemeral decision and choose a stable
  origin source; extending persisted provenance is a separate state-schema
  change and is out of scope.
- Lock-time re-observation can fail after a successful initial observation. The
  operation returns stale/observation attention without writing inventory; the
  caller can retry with a fresh command.
