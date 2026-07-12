---
id: epic-reconciliation-execution-graph
kind: feature
stage: done
tags: []
parent: epic-reconciliation-execution
depends_on: [epic-reconciliation-execution-planner]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Enforce Operation Graph and Selectors

Build dependency-ordered plans over planner operations. Enforce exact
scope-bearing resource/component selectors, include/exclude narrowing,
dependency blockers, and piecewise acknowledgment rules using the existing
operation contract invariants.

## Architectural choice

Add a thin pure graph service around the existing validated `Plan` rather than
reimplementing dependency validation in the CLI or executor. The service
returns deterministic topological waves and applies exact selector filters
before execution. This keeps operation graph invariants centralized and makes
piecewise confirmation independently testable.

## Design decisions

- Dependencies are never silently dropped: selecting an operation includes its
  transitive dependencies unless an explicit exclusion blocks the selection and
  produces an attention finding.
- Include/exclude values match exact `ResourceKey` or component identity; names
  or unqualified ids never cross scopes.
- Acknowledgment selectors must be a subset of the operation's declared
  acknowledgment selectors and consequences must exactly match the plan's
  material consequences. Generic `--yes` is not represented in core.
- Topological ordering is stable by `OperationId` within each ready wave.

## Implementation Units

### Unit 1: Graph traversal and stable waves

**File**: `crates/core/src/operation_graph.rs` (new, exported from
`crates/core/src/lib.rs`)

```rust
pub struct OperationGraph {
    pub plan: Plan,
}

pub struct OperationWave {
    pub operations: Vec<OperationId>,
}

pub fn dependency_waves(plan: &Plan) -> Result<Vec<OperationWave>, GraphError>;
pub fn dependency_closure(plan: &Plan, selected: &BTreeSet<OperationId>)
    -> Result<BTreeSet<OperationId>, GraphError>;
```

Use the already validated `Plan` graph, return all disjoint cycles as typed
errors, and preserve deterministic ordering. A dependency closure never
introduces an unknown operation.

### Unit 2: Exact selectors and acknowledgment gates

**File**: `crates/core/src/operation_graph.rs`

```rust
pub struct OperationSelection {
    pub include: BTreeSet<OperationSelector>,
    pub exclude: BTreeSet<OperationSelector>,
}

pub struct SelectionResult {
    pub plan: Plan,
    pub excluded: BTreeSet<OperationId>,
    pub findings: Vec<GraphFinding>,
}

pub fn select_operations(plan: &Plan, selection: &OperationSelection)
    -> Result<SelectionResult, GraphError>;
pub fn validate_acknowledgment(plan: &Plan, accepted: &AcknowledgmentRequirement)
    -> Result<(), GraphError>;
```

Selection matches resource/component selectors with exact scopes, applies
exclusion precedence, and reports dependency-blocked operations rather than
silently executing an incomplete graph. Acknowledgment validation delegates to
the operation contract and rejects consequence or selector widening.

### Unit 3: Graph contract tests

**Files**: `crates/core/src/operation_graph.rs` tests and
`crates/core/tests/foundation_integration.rs`

Cover multi-wave ordering, disjoint cycles, unknown/self dependencies,
transitive closure, scope collisions, include/exclude precedence, dependency
blockers, exact partial consequence acceptance, and deterministic JSON/Debug
surfaces without raw payloads.

## Implementation Order

1. Implement stable dependency waves and closure over `Plan`.
2. Implement exact resource/component selection and dependency findings.
3. Add acknowledgment gate validation and contract tests.

## Testing

Reuse existing operation fixtures and assert every returned plan remains
constructible through `Plan::new`; no graph helper may mutate repositories or
write output.

## Risks

- Selection closure versus exclusion precedence is subtle; preserve a typed
  dependency-blocked finding rather than guessing whether the user intended to
  omit a prerequisite.
- Keep this feature free of CLI parsing so later commands can share it.

## Implementation notes

- Added `skilltap_core::operation_graph` with deterministic dependency waves,
  transitive dependency closure, exact resource/component selector matching,
  exclusion findings, and acknowledgment validation.
- Graph helpers reuse the validated `Plan` contract and never perform I/O or
  emit terminal output.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core operation_graph --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`

## Review

### Summary

The graph service now supplies stable dependency waves and transitive closure,
exact scope-bearing selector filtering, exclusion findings, and acknowledgment
shape validation on top of the existing validated operation plan.

### Verdict

Approve with comments.

### Findings

- Important follow-up: executor integration must preserve dependency-excluded
  findings as blockers and must not execute a filtered incomplete graph.

### Notes

Inline deep review completed; focused tests and strict clippy pass. CLI parsing
and mutation remain in the dependent features.
