---
id: feature-managed-fallback-target-parity-orchestrator
kind: story
stage: implementing
tags: []
parent: feature-managed-fallback-target-parity
depends_on: [feature-managed-fallback-target-parity-contract, feature-managed-fallback-target-parity-codex-adapter]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Target-Agnostic Managed-Project Orchestrator

## Scope

Implement Unit 3 of the managed-fallback-target-parity feature design:
replace the Codex-specialized CLI orchestrator
`plan_managed_codex_project_lifecycle` with a target-agnostic
`plan_managed_project_lifecycle` that dispatches acquisition and projection
through `adapter.managed_projection()`. Generalize
`validate_managed_project_ownership` to take `&HarnessId` instead of
hardcoding `"codex"`. Flip the `crates/cli/src/application/lifecycle.rs:520`
dispatch site to call the new orchestrator through the registry. Lift
`NativeLifecycleKind` so the port context does not depend on CLI.

After this story, the only `target`-aware line in the managed-project path
is `registry.adapter(target)`; everything below operates on the port and the
resolved `HarnessId`. No `HarnessId::new("codex")` literal remains in CLI.

Parent design: `feature-managed-fallback-target-parity` Unit 3.

## Units

- `crates/cli/src/application.rs` (modified):
  - Delete `plan_managed_codex_project_lifecycle` and
    `ManagedCodexProjectPlanContext`; replace with
    `plan_managed_project_lifecycle` taking
    `registry: &TargetRegistry`, `target: &HarnessId`, and a
    `ManagedProjectPlanContext` (adds `revision_resolver`).
  - Generalize `validate_managed_project_ownership` to take
    `target: &HarnessId`; delete both `HarnessId::new("codex")` literals.
  - Add `From<ManagedPluginWrite> for ManagedProjectPluginWrite` and
    `From<ManagedFileWrite> for ManagedProjectFileWrite` (CLI-private
    translations).
  - Keep `NativeLifecycleKind` as a CLI-internal alias; add
    `From<NativeLifecycleKind> for ManagedLifecycleKind`.
- `crates/cli/src/application/lifecycle.rs` (modified): the dispatch at line
  520 calls `plan_managed_project_lifecycle(&self.registry, target_id, ...)`
  through `adapter.managed_projection()`. The
  `adapter.managed_project_lifecycle() && Scope::Project` gate at line 491
  stays exactly as today (already target-agnostic).

The full code shape (orchestrator body, generalized ownership validation,
dispatch site) is in the parent feature's Unit 3 design body.

## Implementation notes

- The orchestrator is spelled out long-form in the parent design to make the
  target-agnosticism auditable. The implementation should preserve that
  structure: state lookup → port.acquire → observe current →
  validate_managed_project_ownership → port.project → manifest → seed →
  translate to execution-port entry. The only `target`-aware line is the
  top-of-function `registry.adapter(target)`.
- `observe_current_projection_fingerprint` is a small target-agnostic helper
  extracted from the inline observation in the old Codex planner. It
  observes the destination(s) implied by `AcquiredProjection` and returns
  the current fingerprint (`None` when absent). It reuses
  `observe_managed_project_tree` and `managed_project_tree_observation_limits`
  unchanged.
- `plan_as_mcp` rebuilds the MCP `ManagedProjection` entries from the plan's
  file writes + prior projections, preserving the existing
  `managed_projection_manifest` behavior. It is a thin adapter; the heavy
  lifting stays in `managed_projection_manifest`.
- `lifecycle_operation_id` (line 1325) is unchanged; it already takes
  `target: &HarnessId`. The CLI `NativeLifecycleKind` alias keeps operation
  ids stable.
- Ownership validation's signature change is the only API break; the single
  caller is the orchestrator itself, updated in the same commit.
- The foreground acknowledgment gate: the orchestrator must reject any
  `Omitted` entry in the returned plan when `acknowledged == false` with
  `partial_operation_requires_acknowledgment` before producing a plan — even
  though the port contract says adapters should not list omissions when
  unacknowledged. This is defense-in-depth; the existing Codex MCP
  acknowledgment test pins it.
- `3.0.0` is in quality gate. The state shape is unchanged
  (`STATE_SCHEMA_VERSION` stays), so this change must not be cherry-picked
  onto the release branch.

## Acceptance criteria

- [ ] `plan_managed_codex_project_lifecycle` and
      `ManagedCodexProjectPlanContext` no longer exist.
- [ ] `crates/cli/src/application/lifecycle.rs` dispatch (around line 520)
      calls `plan_managed_project_lifecycle` through
      `adapter.managed_projection()`; the line-491 gate is unchanged.
- [ ] `git grep -n 'HarnessId::new("codex")' crates/cli/` returns no matches.
- [ ] `git grep -n '"codex"' crates/cli/src/application.rs` returns no
      behavior-dispatch matches (display labels aside, if any remain).
- [ ] `validate_managed_project_ownership` takes `target: &HarnessId` and
      preserves drift/unowned/update-required/pending-attempt-recovery
      semantics identically.
- [ ] Every existing Codex managed-project test passes without assertion
      changes — the dispatch now flows through the adapter port but
      produces identical operations, entries, seeds, and error codes.
- [ ] A temporary test that registers a throwaway `ManagedProjectionPort`
      adapter for a fake `HarnessId` (e.g. `gemini`) runs the lifecycle and
      produces a planned operation/entry/seed driven entirely by the port
      (this is formalized as the Unit 4 acceptance matrix; here it is the
      minimal proof the orchestrator is target-agnostic).
- [ ] `cargo test --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --all -- --check`, and `git diff --check` pass.

## Out of scope

- The shared acceptance matrix and the formalized fake-adapter profile
  (Unit 4 / `feature-managed-fallback-target-parity-acceptance`). The
  temporary target-agnostic proof here is the minimal version; Unit 4
  expands it into the reusable matrix.
- Any concrete adapter for a new target.
- Claude managed-project lifecycle changes.
