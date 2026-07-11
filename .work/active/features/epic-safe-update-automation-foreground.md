---
id: epic-safe-update-automation-foreground
kind: feature
stage: implementing
tags: []
parent: epic-safe-update-automation
depends_on: [epic-safe-update-automation-policy]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Apply Foreground Updates

Apply safe native-plugin and Git-skill updates with exact acknowledgments and
state recording.

## Architectural choice

Build a pure foreground update planner over resolved candidates and existing
reconciliation `Plan` operations, then send that plan through the existing
configuration lock, revalidation, executor, and state journal. Explicit
foreground commands may request a pinned or partial update, but the request
must carry the exact resource/component selectors emitted by planning. The
daemon will later call the same planner with no acknowledgments, so no daemon
specific mutation path is introduced.

## Design decisions

- **What does foreground scope mean?** A named resource updates only that
  resource; an omitted resource updates all managed resources in the selected
  scopes. Unrelated safe resources may proceed while another remains blocked.
- **How are partial consequences acknowledged?** Reuse the operation-level
  acknowledgment selectors and consequences already validated by the domain.
  A bare global bypass is not accepted; a caller must acknowledge the exact
  consequence set attached to each operation.
- **When is state written?** Only after executor results are terminal and the
  affected resources have been re-observed. A failed or unverified update is
  journaled as attention/partial and never advances the installed revision.

## Implementation Units

### Unit 1: Foreground update plan assembly (trickiest unit)
**File**: `crates/core/src/foreground_update.rs`
**Story**: `epic-safe-update-automation-foreground-plan`

```rust
pub struct ForegroundUpdateRequest<'a> {
    pub resources: &'a [DesiredResource],
    pub candidates: &'a [UpdateCandidate],
    pub mode: UpdateMode,
    pub acknowledgments: &'a BTreeSet<OperationSelector>,
}

pub fn plan_foreground_updates(
    request: ForegroundUpdateRequest<'_>,
) -> Result<ForegroundUpdatePlan, ForegroundUpdatePlanError>;
```

**Implementation Notes**:
- Pair candidates by exact scope-bearing resource key and reject missing or
  duplicate candidates before building operations.
- Emit safe operations for `UpdateSafety::Safe`; retain blocked and
  needs-decision entries as typed findings instead of dropping them.
- Preserve source revision, compatibility summary, and target selectors in
  operation reasons for plain/JSON renderers.

**Acceptance Criteria**:
- [ ] Clean tracked candidates produce deterministic update operations.
- [ ] Blocked/pinned/drifted candidates produce no mutation operation.
- [ ] Acknowledgment selectors are exact and scope-bearing.

### Unit 2: Exact acknowledgment and executor handoff
**File**: `crates/core/src/foreground_update.rs` and `crates/cli/src/application.rs`
**Story**: `epic-safe-update-automation-foreground-acknowledgment`

```rust
pub fn select_foreground_operations(
    plan: &ForegroundUpdatePlan,
    acknowledgments: &BTreeSet<OperationSelector>,
) -> Result<Plan, ForegroundUpdateSelectionError>;
```

**Implementation Notes**:
- Use the existing `Plan`/`Operation` validation and `execute_plan` lock and
  revalidation boundary; do not add a second mutation loop.
- Missing, extra, or cross-scope acknowledgment selectors fail before native
  or filesystem calls. The daemon passes an empty set by design.
- Native plugin updates retain native lifecycle precedence; Git skills retain
  complete-tree and source-SHA semantics.

**Acceptance Criteria**:
- [ ] Repeating a successful foreground plan is a no-op.
- [ ] Partial operations require exact consequence acknowledgment.
- [ ] No native action occurs when selection validation fails.

### Unit 3: Re-observation and state recording
**File**: `crates/core/src/foreground_update.rs` and `crates/cli/src/application.rs`
**Story**: `epic-safe-update-automation-foreground-recording`

```rust
pub trait UpdateResultRecorder {
    fn record_terminal(
        &self,
        plan: &ForegroundUpdatePlan,
        results: &[OperationResult],
    ) -> Result<(), UpdateRecordingError>;
}
```

**Implementation Notes**:
- Record installed revision only after fresh target observations agree with
  the applied plan; preserve available revision and prior apply history.
- Failures retain typed result context for `status` and never overwrite local
  drift or unresolved target disagreement.
- The recorder is a port; application owns repository I/O and atomic state
  publication.

**Acceptance Criteria**:
- [ ] Successful updates advance installed revision and preserve source SHA.
- [ ] Partial/failed updates remain visible without claiming success.
- [ ] State publication is atomic across independent successful resources.

## Implementation Order

1. `epic-safe-update-automation-foreground-plan`
2. `epic-safe-update-automation-foreground-acknowledgment`
3. `epic-safe-update-automation-foreground-recording`

## Testing

- Core decision-table tests cover clean, pinned, drifted, partial, unresolved,
  and duplicate-candidate inputs.
- Executor integration tests prove lock/revalidation behavior and zero native
  calls for invalid acknowledgments.
- State tests prove fresh observation is required before installed revision
  advances and repeat application is idempotent.

## Risks

The riskiest boundary is composing native plugin lifecycle updates and
complete-tree skill updates under one plan without weakening either adapter's
safety contract. The planner therefore emits existing typed operations and
delegates all mutation to the shared executor; it never copies one resource
kind into the other's path or lifecycle.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this
  autopilot run is intentionally single-agent.
