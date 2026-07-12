---
id: epic-cross-harness-materialization-compatibility
kind: feature
stage: done
tags: []
parent: epic-cross-harness-materialization
depends_on: [epic-cross-harness-materialization-graph]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Classify Cross-Harness Compatibility

Produce target-bound faithful, equivalent, partial, unsupported, and conflict
outcomes with exact component consequences and selectors.

## Design decisions

- **How is capability policy kept consistent?** A single core rule table maps
  normalized component kinds to documented capability IDs. Adapters provide
  support evidence; they do not invent equivalence mappings.
- **What does an unknown or unsupported required component do?** It produces a
  target-bound blocked result. Optional unsupported components produce an exact
  partial result with a component selector and consequence.
- **How are identity collisions handled?** A collision in the target's
  occupied component IDs is a conflict even when the component kind is
  otherwise supported. skilltap never renames behavior-bearing identifiers.
- **How do dependencies affect classification?** A component cannot be more
  faithful than any dependency it requires; dependency loss is propagated to
  the dependent component before aggregate results are built.

## Architectural choice

Use a pure compatibility analyzer over `SourceComponentGraph`, target
capability evidence, occupied target identities, and an exact `ResourceKey`.
The analyzer emits one validated `CompatibilityResult` per component plus a
resource aggregate and `OperationSelector` set. A target-specific adapter rule
implementation was rejected because it would scatter policy and make partial
consequences inconsistent. A generic set-intersection heuristic was rejected
because capability support alone cannot express requiredness, identity
collisions, or dependency loss.

## Implementation Units

### Unit 1: Component capability policy and per-component decisions (trickiest unit)
**File**: `crates/core/src/compatibility.rs`
**Story**: `epic-cross-harness-materialization-compatibility-policy`

```rust
pub struct CompatibilityRequest<'a> {
    pub resource: &'a ResourceKey,
    pub graph: &'a SourceComponentGraph,
    pub target: &'a HarnessId,
    pub capabilities: &'a CapabilitySet,
    pub occupied: &'a BTreeSet<ComponentId>,
}

pub struct ComponentDecision {
    pub component: ComponentId,
    pub result: CompatibilityResult,
    pub selector: OperationSelector,
}

pub fn analyze_component(
    request: &CompatibilityRequest<'_>,
    component: &ResourceComponent,
) -> Result<ComponentDecision, CompatibilityAnalysisError>;
```

**Implementation Notes**:
- Define one `COMPONENT_CAPABILITY_RULES` registry for `skill`, `mcp`, `hook`,
  `agent`, `app`, `connector`, `lsp`, command, and other documented kinds.
- Supported capability plus no collision is faithful. Unsupported/unverified
  required capability is blocked; optional capability is partial. A collision
  always uses conflict evidence and blocked fidelity.
- Every non-faithful result is built through `CompatibilityResult::new`, so
  evidence and material consequences are mandatory and target-bound.

**Acceptance Criteria**:
- [ ] Every supported component produces faithful compatibility with no
      consequence; no unsupported required component is marked faithful.
- [ ] Optional unsupported/unverified components produce a partial result with
      an exact affected-component consequence.
- [ ] Occupied identity collisions produce blocked conflict evidence without
      renaming the selector.
- [ ] Unknown component kinds fail closed with a typed evidence result.

### Unit 2: Dependency-aware aggregate and acknowledgment selectors
**File**: `crates/core/src/compatibility.rs`
**Story**: `epic-cross-harness-materialization-compatibility-aggregate`

```rust
pub struct CompatibilityAnalysis {
    pub target: HarnessId,
    pub resource: ResourceKey,
    pub aggregate: CompatibilityResult,
    pub components: BTreeMap<ComponentId, ComponentDecision>,
    pub acknowledgment_selectors: BTreeSet<OperationSelector>,
}

pub fn analyze(
    request: CompatibilityRequest<'_>,
) -> Result<CompatibilityAnalysis, CompatibilityAnalysisError>;
```

**Implementation Notes**:
- Validate graph dependencies before classification and propagate blocked or
  partial dependency outcomes to dependents deterministically.
- Aggregate evidence and consequences in stable `BTreeSet`s. Aggregate
  fidelity is blocked for conflict/required loss, partial when only optional
  loss exists, and faithful only when every component is faithful.
- Selectors are exact `{resource, component_id}` values and include every
  component whose consequence needs acknowledgment. No generic `--yes` scope
  is introduced.

**Acceptance Criteria**:
- [ ] Dependency loss is visible on both the lost component and each affected
      dependent component.
- [ ] Aggregate fidelity/classification cannot claim faithful when any required
      component is blocked or any optional component is omitted.
- [ ] Selector sets are deterministic, scope-bearing, and exactly match the
      aggregate consequences.
- [ ] An unchanged request produces byte-for-byte equal analysis output.

### Unit 3: Reconciliation/planning integration
**File**: `crates/core/src/reconciliation.rs`
**Story**: `epic-cross-harness-materialization-compatibility-integration`

```rust
pub fn compatibility_for_target(
    request: CompatibilityRequest<'_>,
) -> Result<CompatibilityAnalysis, CompatibilityAnalysisError>;
```

**Implementation Notes**:
- Expose the analyzer through the existing core reconciliation boundary so
  later materialization features can convert aggregate results into operation
  classes without importing adapter details.
- Preserve existing `CompatibilityResult` and operation acknowledgment
  invariants; this unit adds no writes and no CLI output.

**Acceptance Criteria**:
- [ ] Reconciliation can consume faithful, materializable/partial, blocked,
      and conflict analyses without reconstructing evidence.
- [ ] Component selectors preserve exact project/global scope through the
      operation planner.
- [ ] No native or managed filesystem operation occurs during classification.

## Implementation Order

1. `epic-cross-harness-materialization-compatibility-policy`
2. `epic-cross-harness-materialization-compatibility-aggregate`
3. `epic-cross-harness-materialization-compatibility-integration`

## Testing

- Core unit tests cover every component-kind rule, all three capability support
  states, requiredness, collisions, and invalid result construction.
- Dependency fixtures cover transitive required/optional loss and deterministic
  selector ordering.
- Reconciliation integration tests prove exact scope-bearing selectors and
  no-write behavior for all aggregate classes.

## Risks

The largest risk is treating an unverified capability as portable because the
component type looks familiar. The analyzer therefore fails closed on absent
or unverified evidence and leaves any faithful-equivalence expansion to a
future rule-table change with new fixture evidence.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this autopilot
  run is intentionally single-agent and no different model was selected.

## Implementation notes

- Completed child stories: `epic-cross-harness-materialization-compatibility-policy`,
  `epic-cross-harness-materialization-compatibility-aggregate`, and
  `epic-cross-harness-materialization-compatibility-integration`.
- Delivered a single capability-rule registry, fail-closed per-component
  results, dependency-loss propagation, collision blocking, exact partial
  selectors, and reconciliation forwarding.
- Verification: targeted core tests and clippy passed; the full workspace
  suite is the final feature review gate.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Deep feature review completed inline in degraded fresh-context mode
because this run is intentionally single-agent. The completeness pass verified
all support states, requiredness, collisions, dependencies, and scope-bearing
selectors. The adversarial pass added propagation evidence for already-partial
dependents and confirmed that no capability or equivalence mapping can silently
grant faithful transfer. Full workspace tests and clippy passed.
