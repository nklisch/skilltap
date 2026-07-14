---
id: epic-expanded-harness-support-native-coexistence-contract
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-native-coexistence
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Route Native and Managed Lifecycle by Evidence

## Checkpoint

Introduce the shared coexistence decision that every native-managed adapter
consumes. Replace project-managed-first routing with a pure, target-neutral
representation selector: existing target state pins updates/removals, a plugin
inherits its exact target-local marketplace representation, and only a fresh
marketplace compares adapter-authored native and managed component plans.

Generalize managed projection from project-only context to one concrete
`Scope`. This lets target adapters project to documented global and project
surfaces without duplicating acquisition, ownership, drift, acknowledgment,
execution, or state logic. Preserve Codex's current global-native/project-
managed behavior exactly.

## Contract

**Files**:

- `crates/core/src/lifecycle_representation.rs` (new),
  `crates/core/src/lib.rs`
- `crates/harnesses/src/native_distribution.rs` (new),
  `crates/harnesses/src/registry.rs`, `crates/harnesses/src/lib.rs`
- `crates/harnesses/src/managed_projection.rs`,
  `crates/harnesses/src/adapters/codex_managed.rs`
- `crates/cli/src/application.rs`,
  `crates/cli/src/application/lifecycle.rs`,
  `crates/cli/src/application/execution.rs`,
  `crates/cli/src/application/tests.rs`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentation { Native, Managed }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepresentationCandidate {
    pub representation: LifecycleRepresentation,
    pub plan: MaterializationPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepresentationEvidence {
    Existing(LifecycleRepresentation),
    Marketplace(LifecycleRepresentation),
    Fresh {
        native: Option<RepresentationCandidate>,
        managed: Option<RepresentationCandidate>,
    },
}

pub fn select_lifecycle_representation(
    evidence: RepresentationEvidence,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;

pub fn applied_lifecycle_representation(
    state: &TargetResourceState,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;
```

```rust
pub struct NativeDistributionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub checkout: &'a ResolvedSourceCheckout,
    pub requested_revision: Option<&'a RequestedRevision>,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

pub trait NativeDistributionPort: Sync {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}
```

Amend `ManagedProjectionContext` to carry `scope: &Scope` instead of
`project: &AbsolutePath`, and replace `plan_managed_project_lifecycle` /
`ManagedProjectPlanContext` with concrete-scope equivalents. Adapters derive all
native roots; core/CLI do not match target ids.

## Required behavior

- Harness-owned native/adopted state routes native. Skilltap-owned materialized
  state with a valid managed manifest routes managed. Contradictory evidence
  fails closed.
- Fresh plugin install follows its target-local marketplace representation.
- Fresh marketplace selection rejects blocked required components. Faithful
  native wins. Managed wins only when native is absent or managed includes a
  strict superset without adding required blocks. Equal partial plans prefer
  native; incomparable partial plans block and expose both consequence sets.
- Once native is selected, an unknown version or narrowed capability blocks the
  native operation; it cannot reselect managed as an authority bypass.
- One resolved checkout is borrowed by native/managed assessors. There is no
  recursive discovery, second clone, or source mutation.
- Existing execution ports, operation dependencies, configuration lock,
  revalidation, rollback, final observation, and target-local state refresh are
  reused.
- `NativeDistributionPort` is an assessment boundary, not a universal plugin
  schema. It emits existing normalized component/materialization evidence while
  each adapter retains its native parser and semantics.

## Acceptance evidence

- Pure tests cover state pinning, marketplace inheritance, native preference,
  managed strict-superset selection, equal/incomparable partial plans, required
  blocks, absent candidates, and contradictory state.
- Mixed-target tests prove one resource may be native on one target and managed
  on another without coalescing ids, revisions, ownership, or journals.
- Codex project managed lifecycle retains operation ids, projection bytes,
  state, removal, pending recovery, and immediate-repeat behavior; Codex global
  lifecycle remains native.
- Managed projection can receive global and project scopes without a target-id
  branch.
- `git grep` finds no new Droid/Qwen/Copilot behavior dispatch in core or CLI.

## Ordering

This is the foundation checkpoint. The Factory, Qwen, and Copilot adapter
stories depend on it; no target profile or registry entry should land before
this route is usable.
