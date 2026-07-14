---
id: feature-daemon-marketplace-refresh
kind: feature
stage: implementing
tags: [infra]
parent: null
depends_on: [epic-safe-update-automation]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-14
---

# Refresh Marketplaces During Daemon Updates

## Brief

Close the omitted first step in the approved daemon update lifecycle. A finite
`skilltap daemon run` currently updates tracked native plugins and Git-backed
skills, but it does not refresh the registered marketplace metadata those
plugin updates resolve through. Build marketplace refresh tasks from managed
inventory and execute them before dependent plugin updates through the same
bounded native lifecycle, lock, postcondition, journal, and status boundaries
used by foreground commands.

The daemon still has no acknowledgment authority. A refresh that is
unsupported, unreachable, drifted, or otherwise requires judgment remains
visible as pending or failed; it cannot silently broaden plugin update
authority. Independent marketplaces may progress incrementally, while a plugin
whose required marketplace refresh did not complete must not update from stale
metadata.

## Strategic decisions

- **Was marketplace refresh intentionally part of daemon updates?** Yes. It is
  the first step in `docs/SPEC.md` and was omitted from the current task
  assembly rather than removed from product scope.
- **What ordering is required?** Refresh each plugin's registered marketplace
  before resolving or applying that plugin update; unrelated resources retain
  incremental progress.
- **Does the daemon gain new authority?** No. It reuses foreground lifecycle
  capabilities with no `--yes` or partial acknowledgment.

## Foundation references

- `docs/SPEC.md` — marketplace lifecycle, plugin update, and daemon interval order.
- `docs/ARCH.md` — shared foreground/daemon application services and locking.
- `docs/UX.md` — unattended update diagnostics and pending decisions.

## Acceptance direction

- A tracked plugin update issues one verified marketplace refresh before its
  native plugin update when the target supports both operations.
- Refresh failure or indeterminate postcondition prevents dependent plugin
  mutation and is recorded for `status` without blocking unrelated resources.
- Duplicate plugins sharing a marketplace do not refresh it redundantly in one
  cycle.
- Repeating a no-change daemon cycle is idempotent and reports no changes.
- JSON and plain output identify refresh, update, pending, and failure results
  without leaking native process output.

## Design decisions

- **Execution boundary**: Build marketplace refreshes and native plugin updates
  into one dependency-aware `Plan` and execute it once through the existing
  hybrid lifecycle port. Chaining separate command outcomes would release the
  configuration lock between refresh and update, would not journal dependency
  skips, and would duplicate executor semantics in the daemon.
- **Dependency identity**: A refresh is unique by exact marketplace
  `ResourceKey` plus `HarnessId`. Scope is therefore part of the key, and a
  refresh completed for one harness or project cannot authorize a sibling
  target or scope.
- **Marketplace selection**: Refresh every registered marketplace whose update
  intent is `Track` on each exact desired target. A tracked plugin derives its
  prerequisite from its exact `plugin@marketplace` selector. A missing,
  untargeted, disabled, pinned, or malformed marketplace relationship leaves
  that plugin update blocked rather than synthesizing registration or widening
  targets.
- **Cycle freshness**: Daemon marketplace refresh and plugin update operations
  are current-cycle attempts, not recovery replays. Prior successful journal
  entries may recover an interrupted foreground mutation, but must not suppress
  a later scheduled refresh or update check.
- **No-change classification**: Marketplace metadata refresh is a prerequisite
  check and does not by itself set the daemon's `changed` summary. For plugin
  updates, compare validated native revision evidence before and after the
  command when the list contract supplies it; equal revisions are `NoChange`,
  changed revisions are `Applied`, and absent revision evidence remains
  conservatively `Applied` rather than claiming a false no-op.
- **Status correlation**: Persist typed references from the last daemon run to
  the ordinary operation journal. `status` resolves those references instead
  of copying errors or native output into a second result store.
- **Dispatch**: Direct-read design was sufficient because the implementation is
  bounded to the existing daemon, lifecycle planner/executor, state journal,
  and isolated native fixtures. The caller explicitly prohibited nested
  advisory agents; implementation remains one cohesive feature-owner bundle
  with stories as checkpoints.

## Architectural choice

Three approaches were considered.

1. **Sequential child commands** would call `marketplace update`, inspect its
   rendered `Outcome`, then call `plugin update`. It is the smallest textual
   change, but each child acquires and releases its own lock, dependency skips
   are not operation-journal facts, and correctness would depend on CLI result
   aggregation rather than the core executor.
2. **One dependency-aware lifecycle plan (chosen)** projects exact marketplace
   and plugin tasks into existing native or managed lifecycle operations, adds
   `OperationDependency` edges, and runs the hybrid port once. It reuses bounded
   argv execution, under-lock revalidation, postconditions, incremental graph
   execution, and `StateExecutionJournal` without creating daemon-only mutation
   authority.
3. **A dedicated daemon updater** could own refresh, resolution, and mutation as
   a new service. It would give a clean file boundary but duplicate foreground
   lifecycle capability checks, managed fallback behavior, and failure
   recovery, contradicting the shared application-service architecture.

The chosen approach makes the task graph pure in core and keeps all external
work behind the existing application and harness ports. The trickiest unit is
extracting reusable exact-target lifecycle planning from
`execute_native_lifecycle` without changing foreground add/remove/update
behavior; that extraction is implemented and parity-tested before daemon batch
composition.

## Implementation Units

### Unit 1: Deterministic daemon native-update graph

**Files**:
- `crates/core/src/daemon.rs`
- `crates/core/src/marketplace.rs`
- `crates/core/src/domain/operation.rs`

**Story**: `feature-daemon-marketplace-refresh-task-graph`

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DaemonMarketplaceRefreshKey {
    resource: ResourceKey,
    target: HarnessId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonMarketplaceRefreshTask {
    key: DaemonMarketplaceRefreshKey,
    name: NativeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonPluginUpdateTask {
    resource: ResourceKey,
    target: HarnessId,
    selector: PluginSelector,
    refresh: DaemonMarketplaceRefreshKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonPluginBlockReason {
    InvalidSelector,
    MarketplaceMissing,
    MarketplaceTargetMissing,
    MarketplaceUpdateDisabled,
    MarketplacePinned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonBlockedPluginUpdate {
    resource: ResourceKey,
    target: HarnessId,
    reason: DaemonPluginBlockReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonNativeUpdatePlan {
    refreshes: Vec<DaemonMarketplaceRefreshTask>,
    plugins: Vec<DaemonPluginUpdateTask>,
    blocked_plugins: Vec<DaemonBlockedPluginUpdate>,
}

pub fn plan_daemon_native_updates<'a>(
    resources: impl IntoIterator<Item = &'a DesiredResource>,
) -> DaemonNativeUpdatePlan;

impl Operation {
    pub fn with_added_dependencies(
        self,
        dependencies: impl IntoIterator<Item = OperationDependency>,
    ) -> Result<Self, OperationContractError>;
}
```

**Implementation notes**:
- Index marketplace resources by exact `ResourceKey`; derive the key for a
  plugin from `PluginSelector::marketplace()` and the plugin's concrete scope.
- Expand both marketplace and plugin resources over their desired target sets.
  Do not use enabled-harness defaults during graph construction.
- Insert refresh tasks into a `BTreeMap<DaemonMarketplaceRefreshKey, _>` so
  plugins sharing a marketplace on the same target/scope get one prerequisite;
  deterministic map order becomes deterministic execution order.
- A marketplace refresh task exists independently of whether it currently has
  a plugin dependent, matching the specification's "refresh registered
  marketplaces" first step. Only `UpdateIntent::Track` is daemon-eligible.
- Invalid or unresolved plugin relationships become typed blocked plugin tasks;
  they do not invalidate unrelated branches of the plan.
- `Operation::with_added_dependencies` unions with any adapter-internal edges
  and rebuilds through `Operation::new`; `Plan::new` remains the cycle and
  dangling-reference validator.

**Acceptance criteria**:
- [ ] Two plugins with the same marketplace, target, and scope produce one
      refresh and two plugin tasks pointing to it.
- [ ] Equal marketplace names in different targets or project scopes produce
      distinct refresh keys.
- [ ] A missing, mismatched-target, pinned, disabled, or malformed prerequisite
      blocks only the affected plugin task.
- [ ] Planning order is stable regardless of inventory serialization order.
- [ ] Added operation dependencies still pass all existing operation contract,
      graph, and wire round-trip tests.

---

### Unit 2: Reusable exact-target lifecycle planning

**Files**:
- `crates/cli/src/application.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application/execution.rs`
- `crates/core/src/lifecycle_operation.rs`

**Story**: `feature-daemon-marketplace-refresh-execution`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LifecycleAttemptPolicy {
    RecoverPriorAttempt,
    ExecuteCurrentCycle,
}

struct NativeLifecyclePlanBuilder {
    operations: Vec<Operation>,
    native_requests: Vec<NativeLifecycleBinding>,
    managed_entries: BTreeMap<OperationId, ManagedProjectLifecycleEntry>,
    seeds: BTreeMap<ResourceKey, ResourceState>,
    foreign_operations: BTreeSet<OperationId>,
}

fn plan_native_lifecycle_target(
    &self,
    builder: &mut NativeLifecyclePlanBuilder,
    documents: &StatusDocuments,
    paths: &PlatformPaths,
    kind: NativeLifecycleKind,
    request: &NativeLifecycleSpec,
    resource: &DesiredResource,
    target: &HarnessId,
    acknowledged: bool,
    attempt: LifecycleAttemptPolicy,
    dependencies: BTreeSet<OperationDependency>,
    outcome: &mut Outcome,
) -> Result<OperationId, ErrorDetail>;

fn execute_native_lifecycle_plan(
    &self,
    command: &'static str,
    documents: &StatusDocuments,
    paths: &PlatformPaths,
    builder: NativeLifecyclePlanBuilder,
    outcome: Outcome,
) -> Outcome;
```

**Implementation notes**:
- Extract the existing exact-target portion of `execute_native_lifecycle` into
  the planner and the existing `Plan`/hybrid-port/journal/lock/report tail into
  the executor. Foreground commands call the same functions with empty
  dependencies and `RecoverPriorAttempt`; their inventory publication,
  acknowledgment, removal, and target-preservation behavior stays unchanged.
- `ExecuteCurrentCycle` bypasses the presence-only prior-journal no-op shortcut
  for update actions. It still performs a fresh pre-observation, capability
  profile selection, exact executable binding, and under-lock revalidation.
- Every requested update produces an operation: executable native/managed,
  verified no-op, or a typed blocked operation. Register blocked operations as
  foreign to the concrete port so revalidation does not expect an executable
  request; the core executor journals them and skips dependents normally.
- Managed project lifecycle operations use the same dependency attachment as
  native operations. No cache write, shell string, or daemon-specific adapter
  is introduced.
- The daemon builds refresh operations first, records their operation IDs by
  `DaemonMarketplaceRefreshKey`, then builds plugin operations with one exact
  `OperationDependency`. It executes the resulting plan once with
  `acknowledged = false`. Git-backed skill updates remain independent existing
  child operations after a normally completed native batch.
- Native command failure or failed/indeterminate postcondition becomes the
  refresh operation's existing typed failed outcome. The executor journals
  dependent plugin operations as `SkippedDependency`; unrelated refresh/plugin
  branches continue. Lock, revalidation, or journal-boundary failure aborts the
  remaining cycle because mutation trust is no longer established.
- Count terminal `Applied`/`NoChange` operations as safe and
  `Failed`/`Blocked`/`SkippedDependency`/`Pending` as pending. Compute daemon
  `changed` from applied plugin/skill resource mutations, excluding successful
  `MarketplaceUpdate` prerequisites.

**Acceptance criteria**:
- [ ] Foreground marketplace/plugin lifecycle tests retain identical authority,
      target, scope, acknowledgment, and recovery behavior after extraction.
- [ ] One configuration lock covers all current-cycle refresh and plugin
      operations, and each plugin operation has the exact refresh dependency in
      the validated `Plan`.
- [ ] A refresh command or postcondition failure journals the refresh failure
      and dependent plugin skip without invoking that plugin command.
- [ ] A failure on one marketplace does not block another marketplace's refresh
      or plugin update.
- [ ] Daemon composition passes `TargetSelection::Only` semantics throughout;
      no unselected target binding or journal is changed.
- [ ] Daemon execution never supplies acknowledgment selectors or `--yes`.

---

### Unit 3: Revision-aware update outcomes

**Files**:
- `crates/harnesses/src/lifecycle.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/cli/src/application/lifecycle.rs`

**Story**: `feature-daemon-marketplace-refresh-execution`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeResourceObservation {
    Present {
        scope: Option<CapabilityScope>,
        revision: Option<ResolvedRevision>,
    },
    Missing,
    Indeterminate(NativeObservationFailure),
}

#[derive(Clone)]
pub struct NativeLifecycleBinding {
    pub operation_id: OperationId,
    pub configured: ConfiguredBinary,
    pub search_path: Option<OsString>,
    pub limits: ProcessLimits,
    pub dispatch: NativeLifecycleDispatch,
    pub before: Option<NativeResourceObservation>,
}

impl NativeLifecyclePort {
    pub fn new_bound_with_environment(
        bindings: impl IntoIterator<Item = NativeLifecycleBinding>,
        environment: BTreeMap<OsString, OsString>,
    ) -> Self;
}
```

**Implementation notes**:
- Extend the strict native list decoder to accept one bounded opaque
  `version` or `revision` scalar on the uniquely matched entry. Conflicting
  fields or duplicate entries remain indeterminate; unknown fields remain
  ignored only according to the existing documented native list envelope.
- After a successful update command and verified presence postcondition,
  `NativeLifecyclePort::apply` returns `NoChange` only when both validated
  before/after revisions exist and are equal. A changed or unavailable
  comparison returns `Applied`; missing/indeterminate postconditions remain
  failures.
- Existing constructors remain as compatibility conveniences with
  `before: None`; the daemon batch uses the bound constructor. This avoids
  changing non-update lifecycle semantics.

**Acceptance criteria**:
- [ ] Equal validated plugin revisions produce `NoChange`; changed revisions
      produce `Applied`.
- [ ] Missing or malformed revision evidence never claims `NoChange` and never
      bypasses the presence postcondition.
- [ ] Scope ambiguity and duplicate identity behavior remain fail-closed.
- [ ] Marketplace refresh success is rendered as a refresh prerequisite and is
      excluded from the daemon resource-change boolean.

---

### Unit 4: Journal-linked daemon status and safe rendering

**Files**:
- `crates/core/src/storage/state.rs`
- `crates/core/src/storage/tests.rs`
- `crates/cli/src/application.rs`
- `crates/cli/src/application/status.rs`
- `crates/cli/src/application/tests.rs`

**Stories**:
- `feature-daemon-marketplace-refresh-execution`
- `feature-daemon-marketplace-refresh-acceptance`

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DaemonOperationRef {
    operation: OperationId,
    resource: ResourceKey,
    target: HarnessId,
    action: OperationAction,
}

impl DaemonOperationRef {
    pub fn new(
        operation: OperationId,
        resource: ResourceKey,
        target: HarnessId,
        action: OperationAction,
    ) -> Result<Self, SchemaError>;
}

impl DaemonRunRecord {
    pub fn with_operations(
        self,
        operations: impl IntoIterator<Item = DaemonOperationRef>,
    ) -> Result<Self, SchemaError>;

    pub fn operations(&self) -> &BTreeSet<DaemonOperationRef>;
}

fn daemon_operation_status_projection(
    state: &StateDocument,
    run: &DaemonRunRecord,
) -> Vec<OutputEntry>;
```

**Implementation notes**:
- Serialize daemon operation references through private `*Wire` DTOs with
  `deny_unknown_fields`, constructor validation, deterministic duplicate
  handling, and a default empty collection for existing state documents.
  Permit only update-cycle actions (`MarketplaceUpdate`, `PluginUpdate`, and
  the existing standalone-skill update action) in this record.
- Build references from the validated plan, never from rendered text. The
  ordinary target-local `ApplyRecord` remains the result source of truth.
- `status` resolves each reference against the exact resource/target journal
  and renders phase (`marketplace_refresh`, `plugin_update`, `skill_update`),
  resource, target, result, and dependency IDs. A missing referenced result is
  `pending/indeterminate`, not success.
- Daemon-run JSON/plain operation entries carry the same typed fields. Rendered
  summaries and evidence codes remain authored and bounded; argv, stdout,
  stderr, source documents, and unknown native payloads never enter state or
  output.

**Acceptance criteria**:
- [ ] `state.json` stores only typed operation references plus the existing
      operation journal, and old documents without references still load.
- [ ] `status --json` identifies a failed marketplace refresh and the exact
      dependent plugin skip from the last daemon run.
- [ ] Plain and JSON daemon output distinguish refreshed, updated, no-change,
      failed, and dependency-pending operations from one shared outcome.
- [ ] Status never attributes one target's result to a sibling target or scope.
- [ ] Raw fake-native output and command arguments are absent from serialized
      state and rendered diagnostics.

---

### Unit 5: Isolated lifecycle acceptance fixture and end-to-end matrix

**Files**:
- `crates/test-support/src/native_process.rs`
- `crates/test-support/src/harness_profile.rs`
- `crates/cli/tests/compiled_binary.rs`
- `crates/cli/tests/native_postconditions.rs`

**Story**: `feature-daemon-marketplace-refresh-acceptance`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeLifecycleAction {
    MarketplaceUpdate,
    PluginUpdate,
}

impl FakeNativeProcess {
    pub fn captured_invocations(&self) -> io::Result<Vec<CapturedInvocation>>;
    pub fn fail_lifecycle(
        &self,
        action: FakeLifecycleAction,
        name: &str,
    ) -> io::Result<()>;
    pub fn set_plugin_revision(&self, selector: &str, revision: &str) -> io::Result<()>;
    pub fn set_available_plugin_revision(
        &self,
        selector: &str,
        revision: &str,
    ) -> io::Result<()>;
}
```

**Implementation notes**:
- Extend the existing test-support-owned fake only; never invoke real Codex,
  Claude, HOME, or XDG state. Record an ordered invocation ledger in the fake's
  temporary root and expose action-specific control files for deterministic
  failure and revision transitions.
- Keep adapter dialect ownership: assertions accept Claude
  `marketplace update` and Codex `marketplace upgrade` as their distinct native
  vectors while checking the same normalized dependency behavior.
- Run mutating scenarios twice and assert the second clean cycle has no plugin
  revision change and `summary.changed == false` even though the registered
  marketplace is refreshed for the new cycle.

**Acceptance criteria**:
- [ ] A tracked plugin on each supported native target records marketplace
      refresh before plugin update using bounded direct argv.
- [ ] Two plugins sharing one exact marketplace produce one refresh invocation
      in the cycle.
- [ ] An injected refresh nonzero exit and an injected indeterminate
      postcondition both suppress the dependent plugin invocation, persist
      status evidence, and allow an unrelated marketplace/plugin branch to
      finish.
- [ ] A multi-target fixture proves target-local failure and sibling progress.
- [ ] Immediate repeat is idempotent and reports no resource changes.
- [ ] Existing foreground lifecycle, postcondition, daemon policy, drift,
      source-failure, and lock-contention regressions remain green.

## Implementation Order

1. `feature-daemon-marketplace-refresh-task-graph` — establish exact task
   identity, deduplication, dependency validation, and operation attachment.
2. `feature-daemon-marketplace-refresh-execution` — extract reusable lifecycle
   planning, compose and execute the one-lock daemon graph, add revision-aware
   no-change outcomes, and persist operation references.
3. `feature-daemon-marketplace-refresh-acceptance` — extend isolated fixtures
   and verify ordering, failure isolation, status, target locality, and repeat
   idempotency end to end.

The feature remains one implementation-owner bundle. The stories are durable
correctness checkpoints, not parallel worker assignments.

## Simplification

- Replace the daemon's ad hoc `(ResourceKind, name, scope)` native-plugin task
  loop with the typed core graph; Git-backed skill dispatch remains in the
  existing path.
- Extract the exact-target lifecycle planner and common execution tail from the
  oversized foreground method instead of copying capability, lock,
  postcondition, and journal code into daemon-specific helpers.
- Reuse `OperationDependency`, `execute_plan`, `NativeLifecyclePort`,
  `ManagedProjectLifecyclePort`, and `StateExecutionJournal`; do not add a
  second executor, daemon acknowledgment model, native cache writer, or result
  journal.
- Retain current foundation documents: they already describe marketplace-first
  daemon order and shared foreground/daemon services as intended truth, so this
  feature creates no documentation drift.

## Testing

- **Pure graph tests** protect exact scope/target identity, selector-derived
  dependencies, deduplication, blocked relationship classification, and stable
  ordering.
- **Executor/application tests** protect one-lock plan construction, dependency
  skip journaling, independent branch progress, no acknowledgment, and
  foreground behavior parity after extraction.
- **Harness contract tests** protect bounded direct argument vectors, strict
  postcondition parsing, revision-aware no-change classification, and target
  scope behavior.
- **Compiled CLI tests** protect the user-visible contract: native ordering,
  per-cycle deduplication, failure isolation, `state.json`/`status` evidence,
  JSON/plain redaction, and immediate-repeat idempotency.
- No test is added for trivial getters or rendering wrappers. Existing tests
  are updated only where the richer `Present { revision }` shape or ordered
  invocation ledger replaces implementation-bound assertions.

## Risks

- **Riskiest assumption**: Native plugin list output exposes a stable installed
  revision on every supported version. The design does not depend on that for
  safety: absent evidence is conservatively `Applied`; only equal validated
  revisions earn `NoChange`. Fixtures cover both paths.
- **Lifecycle extraction regression**: `execute_native_lifecycle` currently
  combines inventory mutation, native and managed routes, recovery no-ops,
  state seeding, execution, and rendering. Extract exact-target planning first
  and require the existing foreground matrix to pass before daemon composition.
- **Managed project behavior**: A project marketplace may use a managed
  documented representation rather than a native command. Dependency edges
  attach after route selection so native and managed refreshes have identical
  executor semantics without pretending one representation is the other.
- **Journal-boundary failure**: A journal error after native apply means the
  machine may have changed without trustworthy state. The cycle stops rather
  than continuing independent work; status retains the daemon run and
  indeterminate operation reference for explicit recovery.
- **Fallback if the batch extraction proves unsafe**: Keep the pure task graph
  and land the planner extraction as the first checkpoint; do not fall back to
  sequential child outcomes, because that would weaken the lock and journal
  guarantees required by the brief.
