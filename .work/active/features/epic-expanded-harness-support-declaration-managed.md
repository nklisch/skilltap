---
id: epic-expanded-harness-support-declaration-managed
kind: feature
stage: done
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Declaration-Managed Target Authority

## Brief

Admit harness capabilities independently by component and concrete scope. A
verified exact-version `Unverified` capability may authorize only a documented,
lossless managed file projection or complete-skill projection as an acknowledged
foreground partial operation. skilltap verifies its owned declaration bytes,
preservation, rollback, conflicts, and disk-level idempotence while reporting
effective load or activation as unverified.

Native lifecycle commands continue to require `Supported`. Unknown versions,
undocumented paths, unmanaged collisions, invalid documents, literal secrets,
interactive authentication or trust, and side-effectful observation never gain
mutation authority. The daemon never acknowledges or applies declaration-managed
operations.

This feature supplies the shared domain, planner, execution, status, and
acceptance contract consumed by the Kiro, Copilot, Kimi, Vibe, Kilo, Junie, Amp,
Cursor, Zoo Code, and ZCode completion checkpoints.

## Foundation decision

The rolling foundation now distinguishes declaration ownership from effective
state. Registration and support are per component and scope rather than
all-or-nothing per target. `Supported` remains ordinary verified authority;
`Unverified` becomes an acknowledgment-gated managed-declaration tier;
`Unsupported` remains non-executable.

## Realized implementation and review

All six checkpoints landed with 704 workspace tests, strict Clippy, formatting,
and diff checks green. The required standard independent review approved the
feature with no material blockers. The implementation retained the private
`ConfiguredAdapterProfile` shape while changing its missing-capability fallback
to `Unsupported`; this is behaviorally equivalent to the proposed replacement
binding and keeps the lock-time equality check explicit.

No canonical adapter exposed a `ManagedDeclarationContract` at feature closure,
so the new declaration-managed path was intentionally dormant and fail-closed.
The first downstream adapter that opts in must add a real compiled-binary
`plugin install --yes` or equivalent end-to-end test; synthetic authority tests
alone are not sufficient. Future target work should also split the currently
coupled `managed.projection` and `component.mcp` profile-helper flags when their
support differs. Internal native filesystem observations remain suitable for
adoption while rendered status labels them `declared`; only a native effective
probe may render effective health.

## Epic context

- Parent epic: `epic-expanded-harness-support`.
- Completed prerequisites: the typed target registry, target-neutral managed
  projection lifecycle, and canonical project-skill link contract.
- Downstream consumers: every pending adapter family now depends on this feature
  so no target invents its own unverified mutator, acknowledgment path, or
  declaration status model.
- Dispatch posture: the six child stories are durable implementation checkpoints
  for one cohesive feature owner. They are not worker-per-story parallelism.

## Foundation references

- `docs/VISION.md` — Native First, Explicit Loss, Observable Ownership, Precise
  Support Over Broad Claims.
- `docs/SPEC.md` — capability-aware planning, foreground `--yes`, declaration-
  managed resources, status, daemon exclusions, ownership, and mutation safety.
- `docs/ARCH.md` — compiled profile authority, declared/effective observation,
  managed projection, revalidated execution, target-local state, and daemon
  reuse of application services.
- `docs/UX.md` — effective-unverified plan/sync language, exact next actions,
  and `--yes` limits.
- `docs/HARNESS-CONTRACTS.md` — per-component/per-scope support, exact-version
  mutation, declaration contract requirements, and native-command restrictions.

The foundation documents already state the intended future contract. This is a
code-first feature; implementation should change a foundation assertion only if
its realized semantics differ, not merely to add coverage.

## Grounding summary

The current code contains most of the required concepts but does not yet connect
them safely:

- `CapabilitySupport::{Supported, Unverified, Unsupported}` already exists in
  `crates/core/src/domain/capability.rs`, and `ScopedCapabilitySets` already
  varies support between global and project scope.
- `CapabilityProfileSelection::mutation_capabilities()` in
  `crates/core/src/domain/installation.rs` correctly returns `None` for unknown
  and verified-observe-only profiles. However
  `configured_adapter_profile` in `crates/cli/src/application.rs` currently
  converts a missing mutation profile or missing capability to `Unverified`.
  That fallback is harmless while managed mutation accepts only `Supported`,
  but would make a direct widening to `Unverified` unsafe for unknown versions.
- Existing compiled helper profiles in
  `crates/harnesses/src/adapter_helpers.rs` primarily enumerate lifecycle and
  `managed.projection` capabilities. They do not consistently carry the
  `skill.*`, `component.skill`, and `component.mcp` decisions needed for
  per-component admission.
- `plan_managed_lifecycle` currently requires
  `profile.capability == CapabilitySupport::Supported`. Route selection also
  consults `HarnessAdapter::supports_managed_projection`, a second coarse
  authority switch that can disagree with the scoped compiled profile.
- `ManagedProjectionPort::plan` already returns only confined complete-tree and
  managed-file writes plus exact manifest/fingerprint evidence. The shared
  `ManagedLifecyclePort` already revalidates the executable/profile and disk
  observations under lock, performs no-follow writes, verifies resulting bytes,
  rolls back captured identities, reports residuals, and returns `NoChange` for
  an immediate repeat. This is the executor declaration management should use.
- Optional compatibility loss is currently gated before operation construction:
  adapters inspect `ManagedProjectionContext::acknowledged`, and standalone
  skills route through a synthetic foreground-update candidate. The validated
  operation model already has `OperationClass::Partial`, exact selectors,
  material consequences, and `AcknowledgmentRequirement`, but the generic
  executor blocks every partial operation and has no exact accepted-
  acknowledgment input.
- Standalone complete-skill installation and project links use
  `ManagedSkillPort`/`ProjectSkillLifecyclePort`, but ordinary adapters do not
  receive the same exact-profile guard as managed plugin projection. The
  conditional-profile filter protects Pi only; a non-conditional target can
  currently reach skill publication without a general exact-version check.
- `ObservationLayer::{Declared, Effective}` already exists. Status nevertheless
  maps coarse filesystem roots and file labels to `Effective` in
  `native_surface_resource`. File presence is therefore over-labeled for targets
  whose runtime state is unavailable. `EffectiveStateProbePort` is the existing
  bounded read-only path for positive effective evidence.
- `TargetResourceState`, `ManagedProjection`, pending managed attempts, and the
  target-local journal already carry sufficient ownership, fingerprints,
  revisions, and retry evidence. No new inventory, executor, or persisted trust
  state is needed.
- The daemon already supplies no acknowledgment and filters managed/native
  capabilities to `Supported` in several places, but route selection can still
  construct a managed candidate before that final check. Declaration-managed
  work needs an explicit pending/skip result before any entry, seed, or native
  process request is built.

Mapping was direct-read only. The caller explicitly prohibited nested agents,
peers, browser work, and authentication.

## Design decisions

- **Exact compiled authority is represented, never inferred.** Add a pure
  mutation-authority function that first requires
  `CapabilityProfileSelection::VerifiedCompiled`. Missing capabilities are
  `Unsupported` for mutation, not `Unverified`; unknown and verified-observe-
  only profiles return a distinct observe-only error. `--yes` cannot alter this
  result.
- **Capability decisions compose by scope and component.** A managed plugin
  requires the scoped `managed.projection` capability plus every included
  component capability derived from the existing component-kind registry
  (`component.skill`, `component.mcp`, and future registered mappings). A
  standalone skill requires its action capability (`skill.install`,
  `skill.update`, or `skill.remove`) plus `component.skill`. Any
  `Unsupported`/missing required capability blocks; any `Unverified` capability
  makes only the affected managed operation declaration-managed; all
  `Supported` capabilities retain ordinary verified behavior.
- **One explicit adapter opt-in bounds `Unverified`.** Extend the existing
  `HarnessAdapter`/managed-projection contract with a scoped
  `ManagedDeclarationContract`. Its closed surface vocabulary permits only
  `ManagedDocument` and `CompleteSkillTree`. The default is absent. An adapter
  with an implementation port but no declaration contract remains blocked when
  any required capability is `Unverified`; this prevents latent or observe-only
  adapters from gaining a mutator accidentally.
- **The plan proves concrete surfaces.** Declaration authorization is evaluated
  after the adapter returns `ManagedProjectionPlan`, against the exact non-empty
  `ManagedFileWrite`/`ManagedPluginWrite` set and projection manifest. Source-
  only marketplace registration may remain a control-plane `NoOp`, but it is
  not itself declaration mutation authority and cannot substitute for a plugin
  projection.
- **Native commands always require `Supported`.** Native lifecycle construction
  uses the same exact-profile function with `MutationChannel::NativeCommand`;
  `Unverified`, absent, unsupported, unknown-version, and observe-only results
  all block even with `--yes`. Runtime probes may narrow the compiled profile
  before this decision and may never widen it.
- **Partial acknowledgment lives in the operation model.** Declaration-only
  effective uncertainty and ordinary optional component omissions become exact
  `MaterialConsequence`s on one `OperationClass::Partial` operation. The
  operation carries the affected resource/component selectors and matching
  `AcknowledgmentRequirement`; preview, foreground sync, and JSON render the
  same plan. Adapters no longer branch on `context.acknowledged`.
- **One executor, with exact foreground authorization.** Extend the existing
  core executor with an `ExecutionAcknowledgments` value that binds an operation
  id to the exact requirement already present in the plan. The existing
  `execute_plan` wrapper supplies none. A foreground `--yes` derives accepted
  entries from the current plan; only those partial operations become
  executable through their existing port. Unsupported/conflict operations can
  never enter the accepted set. The daemon always uses the empty set.
- **Acknowledged partial success remains visible.** Applying a declaration-
  managed operation records ordinary disk ownership and an applied/no-change
  journal result, but the command and subsequent status remain attention-
  required with `declared=healthy` and `effective=unverified`. Acknowledgment
  accepts the consequence; it does not convert it to effective health.
- **Declared and effective observations stay ephemeral and separate.** Reuse
  `ObservationLayer`. Adapter filesystem snapshots and managed-plan removal
  inspection produce `Declared`; only a bounded deterministic
  `EffectiveStateProbePort` result produces `Effective`. An absent or unsafe
  probe produces an explicit effective-unverified row/finding and never reads a
  cache or runs an interactive/trust/auth flow.
- **State schema stays unchanged.** Declaration-managed bytes remain
  `Provenance::Materialized` or `Direct`, `Ownership::Skilltap`, with existing
  target-local fingerprints, manifests, revisions, pending attempts, and apply
  records. Fresh profile/probe evidence determines current effective status, so
  persisting a declaration/effective mode would become stale when a harness
  version changes or gains a verified observer.
- **Collision, preservation, and secret rules are hard blocks.** Adapter codecs
  must continue to reject malformed documents, duplicate/ambiguous native
  locations, same-name unowned entries, higher-precedence shadowing that cannot
  be resolved, literal credentials, and unsupported required components.
  `--yes` accepts only enumerated compatibility/effective-unverified
  consequences; it never approves drift, trust, authentication, secrets, or
  ambiguous ownership.
- **Disk safety is unchanged and mandatory.** Declaration-managed operations
  flow through the existing root-confined filesystem and revalidated execution
  ports. The profile/executable and every file/tree expectation are checked
  again under the lock; rollback restores only captured owned identities and
  reports residuals; immediate repeat returns no change and does not rewrite an
  inode or state record.
- **Daemon work is pending, never attempted.** A daemon cycle classifies any
  operation requiring declaration acknowledgment as pending with an actionable
  reason, builds no managed entry or state seed, and invokes no executor or
  effective probe for it. Supported independent siblings may still proceed.
- **No UI work.** This is a deterministic CLI/domain/adapter contract; no screen
  or flow surface exists, so mockup fallback is skipped.
- **Review posture.** The feature warrants independent design scrutiny because
  it widens mutation authority, but the caller explicitly prohibited nested
  agents and peers. Per policy, design-time advisory review is non-blocking and
  is skipped. Implementation receives the normal feature-level review after
  child verification.

## Architectural choice

**Chosen — exact-profile authorization feeding existing operations and ports.**
Core derives one `MutationAuthorization` from the selected verified compiled
profile, exact scope, required operation/component capabilities, concrete
managed surface kinds, and an optional adapter declaration contract. The
planner turns that result and compatibility consequences into the existing
`Operation` model. The existing executor receives exact foreground
acknowledgments and dispatches the same managed filesystem ports. Status derives
fresh `Declared`/`Effective` observations, and the daemon declines any
acknowledgment-required operation.

**Rejected — a `DeclarationManagedExecutor` or per-target file writer.** A
parallel executor would duplicate plan/apply revalidation, state journals,
ownership, rollback, and idempotence and would invite target-specific safety
variance. The current `ManagedLifecyclePort`, `ManagedSkillPort`, and project-
skill composite already implement the required disk boundary.

**Rejected — relabel `Unverified` as `Supported` after `--yes`.** That would
collapse acknowledgment into authority, admit unknown versions through the
current fallback, permit native commands accidentally, and make status claim
more evidence than exists.

**Rejected — persist a declaration-managed representation enum in state.** The
current profile and effective observer can change independently of owned bytes.
Persisting the mode would duplicate capability truth and require migration when
an adapter gains or loses an effective observer. Existing provenance,
ownership, manifests, and fresh observations are sufficient.

## Trickiest unit first

The riskiest unit is the planner/executor acknowledgment seam. The same partial
operation must be blocked in previews, ordinary sync, and daemon runs, yet become
executable only when a foreground caller accepts the exact selectors and
consequences from that unchanged plan. Solving this in core first prevents CLI
branches from silently rebuilding a safer-looking operation after `--yes` and
keeps every adapter on one execution path.

## Implementation units

### Unit 1: Exact-profile and declaration-surface authority

**Files**:

- `crates/core/src/domain/installation.rs` — exact mutation support lookup.
- `crates/core/src/mutation_authority.rs` (new) and `crates/core/src/lib.rs` —
  pure channel/surface/capability authorization.
- `crates/core/src/compatibility.rs` — expose the existing component-kind to
  capability mapping as the single derivation point.
- `crates/harnesses/src/registry.rs` and
  `crates/harnesses/src/managed_projection.rs` — scoped adapter declaration
  contract; remove the duplicate broad support gate after migration.
- `crates/harnesses/src/adapter_helpers.rs` — profile construction that carries
  explicit lifecycle and component capabilities.

**Story**: `epic-expanded-harness-support-declaration-managed-authority-contract`

```rust
// crates/core/src/domain/installation.rs
impl CapabilityProfileSelection {
    /// `None` means the profile is unknown/observe-only or the capability is
    /// absent. Callers must not reinterpret `None` as `Unverified`.
    pub fn mutation_support(
        &self,
        scope: &Scope,
        capability: &CapabilityId,
    ) -> Option<CapabilitySupport>;
}

// crates/core/src/mutation_authority.rs
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MutationChannel {
    NativeCommand,
    ManagedProjection,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManagedSurfaceKind {
    ManagedDocument,
    CompleteSkillTree,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedDeclarationContract {
    surfaces: BTreeSet<ManagedSurfaceKind>,
}

impl ManagedDeclarationContract {
    pub fn new(
        surfaces: impl IntoIterator<Item = ManagedSurfaceKind>,
    ) -> Result<Self, MutationAuthorityError>;

    pub fn covers(&self, surfaces: &BTreeSet<ManagedSurfaceKind>) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRequirement {
    pub capability: CapabilityId,
    pub affected_components: BTreeSet<ComponentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationAuthorization {
    Supported,
    DeclarationManaged {
        unverified: BTreeSet<CapabilityRequirement>,
    },
}

pub struct MutationAuthorityRequest<'a> {
    pub profile: &'a CapabilityProfileSelection,
    pub scope: &'a Scope,
    pub channel: MutationChannel,
    pub required: &'a [CapabilityRequirement],
    pub surfaces: &'a BTreeSet<ManagedSurfaceKind>,
    pub declaration: Option<&'a ManagedDeclarationContract>,
}

pub fn authorize_mutation(
    request: MutationAuthorityRequest<'_>,
) -> Result<MutationAuthorization, MutationAuthorityError>;

// crates/harnesses/src/registry.rs
pub trait HarnessAdapter: Sync {
    // existing methods
    fn managed_declaration_contract(
        &self,
        scope: CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        None
    }
}
```

**Implementation notes**:

- `authorize_mutation` first rejects any profile whose authority is not
  `VerifiedCompiled`; only then does it inspect scoped support.
- An absent capability has the same mutation result as `Unsupported`, while
  observation may still report it unknown/unverified.
- `NativeCommand` accepts only an all-`Supported` requirement set.
- `ManagedProjection` returns `DeclarationManaged` only when every non-supported
  requirement is exactly `Unverified`, the concrete surface set is non-empty,
  the contract covers all surfaces, and no required capability is absent or
  `Unsupported`.
- Expose the component capability mapping currently private in
  `core::compatibility`; planners and adapters must not create another mapping.
- Delete `HarnessAdapter::supports_managed_projection` after callers migrate.
  Port presence means implementation availability; scoped profile support means
  authority; declaration contract presence means narrow unverified eligibility.

**Acceptance criteria**:

- [ ] Exact known profiles return their real scoped support; unknown and
      verified-observe-only profiles return no mutation support.
- [ ] `Supported` authorizes native or managed channels without acknowledgment.
- [ ] `Unverified` authorizes only covered managed document/complete-tree
      surfaces; it never authorizes a native command.
- [ ] Missing, `Unsupported`, uncovered, empty-surface, wrong-scope, and mixed
      required sets fail closed with typed errors.
- [ ] Global/project and skill/MCP combinations are table-tested independently.
- [ ] Runtime narrowing can change `Supported → Unverified/Unsupported` but can
      never restore or create authority.

---

### Unit 2: Partial operation planning and exact foreground acknowledgment

**Files**:

- `crates/core/src/lifecycle_operation.rs` — construct managed operations from
  `MutationAuthorization` plus compatibility consequences.
- `crates/core/src/domain/operation.rs` and
  `crates/core/src/operation_graph.rs` — exact accepted acknowledgment set.
- `crates/core/src/executor.rs` — authorize partial operations through the
  existing execution loop.
- `crates/harnesses/src/managed_projection.rs` — remove caller acknowledgment
  from adapter planning.
- `crates/cli/src/application.rs`,
  `crates/cli/src/application/lifecycle.rs`, and
  `crates/cli/src/application/project_skills.rs` — build one plan, then derive
  foreground execution acknowledgment from `--yes`.

**Story**: `epic-expanded-harness-support-declaration-managed-planner-acknowledgment`

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionAcknowledgments {
    accepted: BTreeMap<OperationId, AcknowledgmentRequirement>,
}

impl ExecutionAcknowledgments {
    pub fn foreground_all(plan: &Plan) -> Self;

    pub fn new(
        plan: &Plan,
        accepted: impl IntoIterator<
            Item = (OperationId, AcknowledgmentRequirement),
        >,
    ) -> Result<Self, GraphError>;

    pub fn accepts(&self, operation: &Operation) -> bool;
}

pub fn execute_plan_with_acknowledgments<L, P, J>(
    lock: &L,
    lock_path: &AbsolutePath,
    port: &P,
    journal: &J,
    plan: &Plan,
    acknowledgments: &ExecutionAcknowledgments,
) -> Result<ExecutionReport, ExecutionError>;

// Existing callers remain safe by default.
pub fn execute_plan<L, P, J>(...) -> Result<ExecutionReport, ExecutionError> {
    execute_plan_with_acknowledgments(
        ...,
        &ExecutionAcknowledgments::default(),
    )
}
```

**Implementation notes**:

- `managed_materialization_operation` (or a clearer replacement constructor)
  returns `SafeMaterialization` only for `MutationAuthorization::Supported` with
  no material loss. Declaration-managed effective uncertainty or any optional
  omission returns `Partial`, exact evidence/consequences, matching
  `AcknowledgmentRequirement`, and matching attention.
- Declaration consequence code is stable (for example
  `managed.effective_unverified`) and names exact affected components when
  available. Existing omitted-component consequences remain distinct and are
  combined, not replaced.
- `ExecutionAcknowledgments::new` rejects unknown ids, non-partial operations,
  or any requirement unequal to the operation's current requirement.
- The executor treats an accepted `Partial` operation as executable and sends it
  to the same `ExecutionPort`; an unaccepted partial remains blocked. Accepted
  partial results remain attention-required at the aggregate/result rendering
  layer so the consequence stays visible.
- Remove `acknowledged` from `ManagedProjectionContext`. Adapters always report
  optional omissions in their manifest and always return
  `RequiredUnsupported` for required loss. The planner owns acknowledgment.
- Preview and sync construct byte-for-byte equal operations. `--yes` changes
  only `ExecutionAcknowledgments`; it does not rebuild compatibility or widen
  authority.
- Do not persist inventory or state before an unacknowledged partial is known to
  be executable; a blocked foreground attempt must leave both control-plane and
  target bytes unchanged.

**Acceptance criteria**:

- [ ] A declaration-only plan is a validated `Partial` operation with exact
      file/tree surfaces, selectors, evidence, consequences, and acknowledgment.
- [ ] No `--yes` blocks and journals no pending mutation; `--yes` applies the
      exact same operation through the existing port.
- [ ] Changed/missing/extra acknowledgment requirements are rejected.
- [ ] `--yes` never executes `Unsupported`, `Conflict`, native `Unverified`,
      drifted, trust/auth-required, or required-component-blocked operations.
- [ ] Existing optional component and non-strict-skill acknowledgments migrate
      to the operation model without synthetic revision candidates or
      adapter-local acknowledgment branches.
- [ ] Independent supported siblings continue when one partial operation is
      blocked; dependencies of the blocked operation skip normally.

---

### Unit 3: Revalidated managed execution and declared/effective status

**Files**:

- `crates/cli/src/application.rs` — exact `MutationProfileBinding`, managed
  authority assembly, and declaration inspection.
- `crates/cli/src/application/execution.rs` — profile guards for managed
  plugin, global skill, and project-skill execution ports.
- `crates/cli/src/application/lifecycle.rs` and
  `crates/cli/src/application/project_skills.rs` — pass exact profile bindings
  and render acknowledged partial outcomes.
- `crates/cli/src/application/status.rs` — semantic managed-resource projection,
  corrected observation layers, and effective-unverified output.
- `crates/core/src/domain/resource/finding.rs` — only the minimal registered
  finding/summary additions not already covered by `CapabilityUnverified`,
  drift, conflict, trust, and consent.

**Story**: `epic-expanded-harness-support-declaration-managed-execution-status`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
struct MutationProfileBinding {
    target: HarnessId,
    scope: Scope,
    configured: ConfiguredBinary,
    executable: ExecutableIdentity,
    native_version: NativeVersion,
    profile: CapabilityProfileSelection,
    requirements: Vec<CapabilityRequirement>,
    authorization: MutationAuthorization,
}

fn configured_mutation_profile(
    registry: &TargetRegistry,
    config: &ConfigDocument,
    target: &HarnessId,
    scope: &Scope,
    requirements: &[CapabilityRequirement],
    surfaces: &BTreeSet<ManagedSurfaceKind>,
    channel: MutationChannel,
    runtime: NativeProfileRuntime<'_>,
) -> Result<MutationProfileBinding, MutationProfileError>;

fn managed_status_projection(
    registry: &TargetRegistry,
    documents: &StatusDocuments,
    scopes: &StatusScope,
    targets: &StatusTargets,
    filesystem: &dyn ManagedLifecycleFileSystem,
) -> ManagedStatusProjection;
```

**Implementation notes**:

- Replace `ConfiguredAdapterProfile.capability` and its default-`Unverified`
  fallback with a binding created only after exact authority is evaluated.
- Attach the binding to every managed execution entry, including standalone
  complete skills and project canonical/link operations. Under the lock,
  re-detect the same executable identity/version/profile, reapply runtime
  narrowing, and require the authorization result to equal the planned result
  before any write.
- Keep existing file/tree expectation revalidation, no-follow mutation,
  post-write byte/tree verification, target-local state seeding, pending retry,
  identity-aware rollback, and residual reporting unchanged.
- For existing managed bindings, status can call the pure
  `ManagedProjectionPort::plan` with `ManagedProjectionInput::Remove` and the
  recorded prior manifest to inspect owned declarations without source
  acquisition or mutation. Standalone skill status uses its existing bounded
  complete-tree observation.
- Filesystem roots/settings are `Declared`. Only a deterministic bounded native
  probe yields `Effective`. Declaration-managed support emits an explicit
  effective unknown/unverified resource or field plus a registered capability
  finding; it never calls an interactive or side-effectful probe.
- Supported profiles continue to use `EffectiveStateProbePort` where required.
  Trust, auth, disabled, failed, and reload-required states are attention health,
  not file drift and not mutation authority.
- Plain and JSON output derive from one outcome and include enough fields to
  distinguish `declared=healthy|drifted|conflict` from
  `effective=healthy|unverified|trust_required|authentication_required|failed`.
- Keep `STATE_SCHEMA_VERSION` unchanged. Status derives the current mode from
  fresh profile/probe evidence and existing target-local ownership.

**Acceptance criteria**:

- [ ] Executable identity, exact native version, scoped profile, component
      requirements, declaration contract, and expected disk identities are all
      revalidated under lock before an accepted partial write.
- [ ] File/tree apply, update, remove, rollback, pending recovery, target-local
      sibling preservation, and immediate-repeat no-change behavior remain
      identical for Supported and declaration-managed paths.
- [ ] Correct declaration bytes never become an `Effective` healthy resource
      without positive bounded native evidence.
- [ ] Declaration-only foreground success records owned bytes and remains
      attention-required with explicit effective-unverified status.
- [ ] Ambiguous/same-name unowned/higher-precedence collisions, malformed
      documents, literal secrets, drift, and replacement races remain blocked
      with all unrelated bytes intact.
- [ ] Unknown or changed versions between plan and apply fail revalidation even
      when `--yes` was supplied.

---

### Unit 4: Daemon exclusion and pending update semantics

**Files**:

- `crates/core/src/daemon.rs` — typed pending reason for acknowledgment-required
  declaration updates.
- `crates/cli/src/application/lifecycle.rs` — classify before daemon route/entry
  construction.
- `crates/cli/src/application/status.rs` — render pending declaration-managed
  updates and last-run evidence.
- `crates/cli/tests/compiled_binary.rs` — daemon no-write and sibling-progress
  regression coverage.

**Story**: `epic-expanded-harness-support-declaration-managed-daemon-safety`

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DaemonPendingReason {
    AcknowledgmentRequired,
    DeclarationManaged,
    Drifted,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonPendingUpdate {
    pub resource: ResourceKey,
    pub target: HarnessId,
    pub reason: DaemonPendingReason,
}
```

**Implementation notes**:

- Resolve exact profile/capability authority before constructing a daemon
  managed/native request. `DeclarationManaged` becomes pending immediately.
- Build no managed entry, state seed, operation journal, checkout mutation, or
  effective probe for declaration-managed work.
- The daemon continues to execute independent Supported operations in dependency
  order. A declaration-managed prerequisite keeps only its dependents pending.
- `daemon run` never calls `ExecutionAcknowledgments::foreground_all` and cannot
  receive `--yes` from CLI parsing.
- Pending counts/status name the resource, target, scope, and reason without
  exposing raw paths, argv, document bytes, or secrets.

**Acceptance criteria**:

- [ ] A declaration-managed install/update/remove is pending in every daemon
      mode and leaves target files, inventory, state bindings, and operation
      journals unchanged.
- [ ] A Supported sibling still applies; a dependent operation skips/pends
      without broad cycle failure.
- [ ] Unknown-version, drifted, conflict, required-unsupported, trust/auth, and
      declaration-managed cases are never converted into safe daemon work.
- [ ] Repeated daemon runs remain byte-for-byte no-ops for pending declaration
      work and preserve a stable actionable status result.

---

### Unit 5: Existing-path migration and regression preservation

**Files**:

- Current adapters and profiles under `crates/harnesses/src/adapters/` and
  `crates/harnesses/src/adapter_helpers.rs`.
- `crates/cli/src/application.rs`, `application/lifecycle.rs`,
  `application/project_skills.rs`, and `application/execution.rs`.
- Existing core/harness/CLI tests and schema fixtures.

**Story**: `epic-expanded-harness-support-declaration-managed-migration-regressions`

**Implementation notes**:

- Expand each exact compiled profile to state `skill.*`,
  `component.skill`, `component.mcp`, and `managed.projection` independently for
  global/project scope. Do not infer one from another and do not change unknown-
  version observe-only behavior.
- Migrate Codex, Factory, Gemini, OpenCode, Qwen, and any landed Kiro path from
  `ManagedProjectionContext::acknowledged` to unconditional omission evidence.
  Only adapters with attested declaration surfaces add a
  `ManagedDeclarationContract`; all defaults remain blocked.
- Remove `supports_managed_projection` and every profile fallback that treats
  absence as unverified mutation support.
- Migrate standalone global skills and project canonical/link paths onto exact
  profile bindings while preserving their existing ownership, all-target shared
  content rule, conflict handling, and global-versus-project representation.
- Preserve existing Supported behavior: current exact Codex/Claude/native and
  effectively verified managed operations do not acquire a new `--yes`
  requirement; native lifecycle preference and applied representation pinning
  remain unchanged.
- Keep `inventory.toml` and `state.json` schemas unchanged. Existing
  `Materialized`/`Direct` target bindings load without a migration layer;
  declaration/effective status is derived fresh.
- Retain accepted consequence wire validation. Replace synthetic standalone
  foreground-update acknowledgment only after operation-level tests prove the
  same or stronger selector/consequence contract.

**Acceptance criteria**:

- [ ] Existing schema fixtures deserialize and round-trip identically; schema
      versions do not change and no compatibility loader is added.
- [ ] Existing Supported exact-version operations remain safe/no-ack, native-
      first, target-local, rollback-safe, and idempotent.
- [ ] Existing unknown/adjacent versions remain no-write even with `--yes` for
      explicit install/update/remove, `sync`, project links, and daemon runs.
- [ ] Existing optional omission/non-strict skill flows show the same material
      loss through validated partial operations, without weakening tests.
- [ ] No adapter lacking an explicit declaration contract gains an unverified
      write route merely because it exposes a managed port.
- [ ] Greps find no `supports_managed_projection`, default mutation
      `unwrap_or(CapabilitySupport::Unverified)`, adapter-local acknowledgment
      gate, or synthetic partial revision marker after migration.

---

### Unit 6: Integrated declaration-managed acceptance matrix

**Files**:

- `crates/test-support/src/managed_acceptance.rs` and
  `crates/test-support/src/harness_profile.rs` — dependency-neutral scenarios
  and profile declaration surface descriptors.
- `crates/core/src/mutation_authority.rs`, `domain/operation/tests.rs`, and
  `executor.rs` tests — pure authority/acknowledgment contracts.
- `crates/cli/src/application/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` — production-aware lifecycle/status/
  daemon acceptance.

**Story**: `epic-expanded-harness-support-declaration-managed-acceptance`

**Implementation notes**:

- Extend the shared matrix rather than creating a declaration-only test runner.
  Profile fixtures declare exact global/project support and allowed managed
  surface kinds; production-aware callbacks perform real assertions before
  returning evidence labels.
- Exercise one fake exact profile with Supported skill + Unverified MCP, one
  all-Unverified managed profile, one Unsupported component, one uncontracted
  port, and an adjacent unknown version.
- Run explicit plugin/skill lifecycle, `plan`, `sync`, status, update/remove,
  pending recovery, rollback failure, and daemon cases in isolated HOME/XDG/
  project roots. Never invoke real harnesses or operator state.

**Acceptance criteria**:

- [ ] `Supported` is ordinary verified; declaration `Unverified` blocks without
      `--yes`, applies only with foreground `--yes`, and `Unsupported` blocks in
      both cases.
- [ ] Native commands require `Supported` under the same exact profile; an
      unverified native capability cannot be acknowledged.
- [ ] Every capability and result is independent by global/project scope and by
      skill/MCP component; one unsupported sibling does not erase a safe one,
      while a required unsupported dependency blocks its resource.
- [ ] Plan/JSON show exact files, complete-skill roots, selectors,
      reversibility, omitted/effective-unverified consequences, and next action.
- [ ] Status shows declared ownership/health separately from effective
      unverified state and never reports loaded/healthy from file presence.
- [ ] Same-name unmanaged entries, ambiguous precedence, malformed documents,
      literal secrets, drift, and races produce no write; unrelated fields and
      sibling resources remain byte-for-byte intact.
- [ ] Accepted operations prove disk verification, ownership-safe update/remove,
      pending retry, rollback/residual reporting, target isolation, and
      immediate-repeat no file/inode/plan/state change.
- [ ] Daemon runs skip declaration-managed operations and may apply independent
      Supported work.
- [ ] Unknown/adjacent versions never mutate through any command, even with
      `--yes`.
- [ ] Current Codex/Claude and landed managed-adapter regression suites pass;
      workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` are green.

## Realized implementation

All six implementation checkpoints are complete. The shared authority now
requires exact scoped profiles and narrow declaration contracts; partial managed
outcomes carry validated selectors and consequences through the existing lock,
journal, rollback, and execution ports; status separates declared ownership
from effective-unverified evidence; daemon reconciliation skips declaration-
managed work; and existing standalone/adaptor paths use exact profile bindings.

Verification is green:

- `cargo test --workspace --all-targets` — 704 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean.
- `git diff --check` — clean.
- Existing compiled acceptance matrix — 69 passed.

The feature is committed and ready for the requested parent review pass.

## Implementation order

1. `epic-expanded-harness-support-declaration-managed-authority-contract` —
   `depends_on: []`.
2. `epic-expanded-harness-support-declaration-managed-planner-acknowledgment` —
   `depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract]`.
3. `epic-expanded-harness-support-declaration-managed-execution-status` —
   `depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract,
   epic-expanded-harness-support-declaration-managed-planner-acknowledgment]`.
4. `epic-expanded-harness-support-declaration-managed-daemon-safety` —
   `depends_on: [epic-expanded-harness-support-declaration-managed-planner-acknowledgment,
   epic-expanded-harness-support-declaration-managed-execution-status]`.
5. `epic-expanded-harness-support-declaration-managed-migration-regressions` —
   `depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract,
   epic-expanded-harness-support-declaration-managed-planner-acknowledgment,
   epic-expanded-harness-support-declaration-managed-execution-status]`.
6. `epic-expanded-harness-support-declaration-managed-acceptance` —
   `depends_on: [epic-expanded-harness-support-declaration-managed-daemon-safety,
   epic-expanded-harness-support-declaration-managed-migration-regressions]`.

`work-view --blocking` was run for every new story receiving a sibling
`depends_on` entry before these edges were written; no existing dependents were
reported, so the graph introduces no cycle.

## Simplification

- Eliminate `HarnessAdapter::supports_managed_projection`; port availability,
  exact scoped capability support, and a narrow declaration contract become the
  three non-duplicated facts.
- Eliminate mutation-support fallback to `Unverified`; unknown/absent remains
  structurally unable to authorize mutation.
- Eliminate `ManagedProjectionContext::acknowledged` and adapter-local partial
  gates; adapters classify, operations describe consequences, foreground
  execution acknowledges.
- Eliminate the standalone skill's synthetic revision pair used only to reach
  foreground acknowledgment; use the same partial operation contract.
- Reuse `ManagedLifecyclePort`, `ManagedSkillPort`, project-skill execution,
  target-local state, pending attempts, and rollback. Add no declaration
  executor, target-family dispatcher, persisted trust state, or second
  capability registry.
- Correct coarse filesystem status from `Effective` to `Declared`; retain
  `EffectiveStateProbePort` as the sole positive native effective evidence.

No separate refactor/cleanup story is warranted. Each deletion is coupled to
the contract that replaces it and is verified in the migration checkpoint.

## Testing

- **Pure authority tables:** exact/observe-only/unknown profile × global/project
  × native/managed × Supported/Unverified/Unsupported × documented/undocumented
  surface. Protects the mutation ceiling.
- **Operation/executor tests:** exact consequence coverage, mismatched accepted
  requirements, partial blocked versus accepted execution, dependency skips,
  and aggregate attention. Protects the acknowledgment seam.
- **Port regression tests:** existing tree/file revalidation, same-name
  ownership conflicts, atomic preservation, rollback identities/residuals,
  pending attempts, and target-local state. Protects disk safety without
  retesting trivial getters.
- **Status tests:** declared/effective layering, profile change, missing probe,
  trust/auth/reload health, drift, and plain/JSON parity. Protects truthful
  claims.
- **Daemon tests:** pending declaration work, supported sibling progress, no
  acknowledgment path, and immediate-repeat no writes.
- **Migration tests:** existing wire fixtures and Supported adapter behavior,
  plus `--yes` unknown-version no-write regressions on every mutating command
  family.
- **Integrated acceptance:** isolated global/project complete skills and MCP
  declarations, mixed component support, ownership/removal, collision,
  preservation, secret rejection, rollback, idempotence, and daemon skip.

Low-value tests are not added for static contract getters, every profile map
entry in isolation, every renderer string, or target-specific document details
owned by downstream adapter features. The matrix pins stable authority,
operation, surface, finding, and externally visible outcomes.

## Risks

- **Riskiest assumption — accepted partial operations can remain the same
  validated `Operation`.** The current executor blocks every partial before
  port dispatch. The selected extension passes an exact accepted-requirement
  set into that executor and permits only matching partial operations. If the
  operation-result contract exposes an unforeseen invariant, the fallback is a
  pure authorization wrapper around the same plan/port—not a second executor or
  a rebuilt safe operation.
- **Capability-set migration can accidentally remove current support.** Existing
  profiles omit several newly required component/lifecycle ids. Migration uses
  explicit per-adapter tables and current behavior regression tests; absence
  blocks rather than silently becoming unverified. Downstream targets add
  capabilities only with exact-version evidence.
- **Adapter plan evidence may not identify every affected component.** Skill and
  MCP manifests already provide stable ids. If a target stores multiple MCP
  servers in one document or skill-local MCP in a tree, consequences remain
  component-specific while affected surfaces stay file/tree-specific. A
  resource-level selector is the conservative fallback when no safe component
  id exists; inventing an id is not.
- **Status inspection through removal planning could couple read-only status to
  lifecycle assumptions.** `ManagedProjectionInput::Remove` is already
  source-free, read-only during planning, and driven by prior manifest evidence.
  If an adapter cannot safely expose current declaration evidence this way, add
  an `inspect` method to the same `ManagedProjectionPort`; do not create a
  parallel status adapter or parse target documents in CLI.
- **A correctly written declaration may remain ineffective indefinitely.** That
  is the intended partial consequence. Status stays attention-required and the
  daemon keeps updates pending; skilltap never upgrades the claim from time,
  file presence, cache contents, or user acknowledgment.
- **Profile changes after installation.** Fresh status may move an owned
  declaration between effective-verified and effective-unverified as versions
  change. State remains ownership evidence, not authority. Every later mutation,
  including removal, requires a currently exact known profile; an unknown
  version leaves owned bytes in place for manual recovery rather than mutating
  under stale authority.
- **Migration breadth.** Profiles, managed plugins, standalone skills, status,
  executor authorization, and daemon policy cross several modules. The child
  order lands the pure ceiling first, then operation semantics, then execution
  and status, with migration and integrated acceptance last. If a later unit
  fails, the safe fallback is the current Supported-only behavior, not a partial
  widening.
