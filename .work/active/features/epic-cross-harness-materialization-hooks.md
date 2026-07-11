---
id: epic-cross-harness-materialization-hooks
kind: feature
stage: implementing
tags: []
parent: epic-cross-harness-materialization
depends_on: [epic-cross-harness-materialization-compatibility]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Materialize Hooks and Target Components

Map documented hooks and target-specific components only when behavior remains
faithful; unsupported required behavior blocks the resource.

## Design decisions

- **What proves hook equivalence?** The normalized contract must compare event
  timing, payload shape, failure behavior, working directory, environment
  references, and executable permission semantics. Similar event names alone
  never qualify.
- **How are unknown events handled?** Required unknown events block the target;
  optional unknown events become explicit partial consequences. No user mapping
  can relabel an event as faithful.
- **Where are hook files written?** This feature emits a target-bound mapping;
  the publish feature owns complete-tree publication and native registration.

## Architectural choice

Use a normalized `HookContract` at the core boundary and adapter-owned parsers
for Codex/Claude hook manifests. A raw JSON pass-through would hide payload and
failure mismatches, while target-specific copy code would bypass the existing
compatibility/evidence contract. The chosen design validates bounded command,
event, environment-reference, and permission metadata once, then returns the
existing target-bound compatibility result.

## Implementation Units

### Unit 1: Normalized hook contract and adapter readers (trickiest unit)
**File**: `crates/core/src/hook_mapping.rs` and `crates/harnesses/src/hook_mapping.rs`
**Story**: `epic-cross-harness-materialization-hooks-contract`

```rust
pub struct HookContract {
    pub component: ComponentId,
    pub event: String,
    pub payload: HookPayload,
    pub failure: HookFailure,
    pub working_directory: HookWorkingDirectory,
    pub environment_references: BTreeSet<String>,
    pub executable: bool,
}

pub trait HookContractReader {
    fn read(
        &self,
        graph: &SourceComponentGraph,
        component: &ComponentId,
    ) -> Result<HookContract, HookMappingError>;
}
```

**Implementation Notes**:
- Use bounded strict JSON/TOML parsing in adapters and retain only normalized
  event/payload/failure/path/permission values; never retain raw commands with
  secret arguments or raw native documents.
- `HookPayload` and `HookFailure` use closed validated enums where the native
  contracts are closed; unknown documented extensions remain an explicit
  unknown value and classify fail-closed.

**Acceptance Criteria**:
- [ ] Codex and Claude fixtures produce normalized hook contracts with stable
      environment-reference sets and no secret values.
- [ ] Malformed manifests, missing hook files, and unsafe paths return typed
      errors before mapping.
- [ ] Reader operations are bounded and observation-only.

### Unit 2: Hook equivalence analysis
**File**: `crates/core/src/hook_mapping.rs`
**Story**: `epic-cross-harness-materialization-hooks-equivalence`

```rust
pub fn analyze_hook(
    source: &HookContract,
    target: &HookTargetContract,
    requiredness: ComponentRequiredness,
    target_harness: &HarnessId,
    resource: &ResourceKey,
) -> Result<CompatibilityResult, HookMappingError>;
```

**Implementation Notes**:
- Compare every contract field, including payload and failure semantics, and
  emit one typed evidence code per mismatch with the exact component affected.
- Required mismatches are blocked; optional mismatches are partial with a
  material consequence. Exact component selectors remain scope-bearing.
- A faithful result has no consequence; all other results use the existing
  `CompatibilityResult` constructor.

**Acceptance Criteria**:
- [ ] Identical contracts classify faithful.
- [ ] Any event, payload, failure, cwd, environment, or permission mismatch is
      visible and cannot classify faithful.
- [ ] Required and optional mismatch paths produce blocked versus partial
      fidelity with exact evidence/consequence sets.

### Unit 3: Materialization/reconciliation handoff
**File**: `crates/core/src/reconciliation.rs`
**Story**: `epic-cross-harness-materialization-hooks-integration`

```rust
pub fn hook_compatibility_for_target(
    source: &HookContract,
    target: &HookTargetContract,
    requiredness: ComponentRequiredness,
    target_harness: &HarnessId,
    resource: &ResourceKey,
) -> Result<CompatibilityResult, HookMappingError>;
```

**Implementation Notes**:
- Keep hook analysis pure and compose its result with the existing component
  aggregate before any publication or native registration.
- Preserve exact resource scope and component selector identity through the
  operation planner.

**Acceptance Criteria**:
- [ ] Unsupported required hooks block the plugin before publication.
- [ ] Optional hook omissions remain visible to partial acknowledgment planning.
- [ ] No native lifecycle or managed filesystem operation runs during mapping.

## Implementation Order

1. `epic-cross-harness-materialization-hooks-contract`
2. `epic-cross-harness-materialization-hooks-equivalence`
3. `epic-cross-harness-materialization-hooks-integration`

## Testing

- Fixture tests cover equivalent contracts and each independent mismatch field.
- Security-facing fixtures assert no command arguments, environment values, or
  native payload bytes cross the core boundary.
- Reconciliation tests prove exact blocked/partial selectors and no-write
  behavior.

## Risks

Hook runtimes may add events or payload fields without a stable documented
schema. Unknown fields remain unverified and block required transfers until an
adapter contract is updated with fixture evidence; the mapper never guesses.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this autopilot
  run is intentionally single-agent and no different model was selected.
