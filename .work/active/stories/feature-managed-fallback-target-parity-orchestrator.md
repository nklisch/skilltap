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

Implement Unit 3 of the managed-fallback-target-parity feature design against
its **amended** contract: replace the Codex-specialized CLI orchestrator
`plan_managed_codex_project_lifecycle` with a target-agnostic
`plan_managed_project_lifecycle` that resolves source once in CLI, builds one
`ManagedProjectionContext`, and calls the selected adapter's
`ManagedProjectionPort::plan` method. Generalize
`validate_managed_project_ownership` to take `&HarnessId` instead of
hardcoding `"codex"`. Flip the
`crates/cli/src/application/lifecycle.rs:520` dispatch site to call the new
orchestrator through the registry.

This story deliberately supersedes the earlier Unit 3 acquire/project
instructions in the parent design. The active port surface is:

- `ManagedProjectionInput::{Apply { checkout }, Remove}`.
- `ManagedProjectionContext { target, project, paths, resource_key,
  resource_kind, request, kind, input, prior, acknowledged, filesystem,
  json_limits }`.
- `ManagedProjectionPort::plan(&ManagedProjectionContext) ->
  ManagedProjectionPlan`.
- `ManagedProjectionPlan { trees, files, manifest, current_fingerprint,
  desired_fingerprint }`.

After this story, no `HarnessId::new("codex")` literal remains in CLI managed
project orchestration, and the CLI does not reconstruct target-native
projection evidence from file writes.

Parent design: `feature-managed-fallback-target-parity` Unit 3, corrected by
the `feature-managed-fallback-target-parity-contract-evidence` amendment and
the completed `feature-managed-fallback-target-parity-codex-adapter` story.

## Units

- `crates/cli/src/application.rs` (modified):
  - Delete `plan_managed_codex_project_lifecycle` and
    `ManagedCodexProjectPlanContext`; replace with
    `plan_managed_project_lifecycle` taking `registry: &TargetRegistry`,
    `target: &HarnessId`, and a target-neutral `ManagedProjectPlanContext`.
  - Keep source resolution in CLI orchestration via a generic
    `resolve_managed_source_checkout(paths, source)` helper that reuses the
    existing `resolve_git_skill_source` machinery for Git sources and validates
    local sources into `ResolvedSourceCheckout`.
  - Generalize `validate_managed_project_ownership` to take
    `target: &HarnessId`; delete both `HarnessId::new("codex")` ownership
    literals.
  - Keep the existing mechanical translations from `ManagedFileWrite` /
    `ManagedPluginWrite` into CLI-private execution-port writes.
  - Keep `NativeLifecycleKind` CLI-internal and convert it to
    `ManagedLifecycleKind` at the port boundary.
- `crates/cli/src/application/lifecycle.rs` (modified): the dispatch around
  line 520 calls `plan_managed_project_lifecycle(&self.registry, target_id,
  ...)` after resolving `adapter.managed_projection()`. The existing
  `adapter.managed_project_lifecycle() && Scope::Project` gate stays in
  place; it was already target-agnostic.

## Target-neutral flow

Implement the orchestrator in this order so ownership and source lifetimes are
explicit and no target-specific CLI side channel is needed:

1. **Resolve the adapter and port.** Look up `registry.adapter(target)` and
   then `adapter.managed_projection()`. Return typed attention errors for an
   unregistered target or a target that does not provide the port.
2. **Derive shared lifecycle state.** Build the project scope, operation id,
   existing resource state, target-local state, pending-attempt-aware
   `prior_projections`, and `removal = matches!(kind,
   MarketplaceRemove | PluginRemove)` exactly once for the resolved target.
3. **Decide `Apply` vs `Remove` from lifecycle kind.**
   - For `MarketplaceAdd` / `MarketplaceUpdate`, resolve the marketplace
     resource's one authoritative source (`resource.source()` first, then the
     existing target state's source) into `ResolvedSourceCheckout`.
   - For `PluginInstall` / `PluginUpdate`, parse the selected plugin's
     marketplace identity from the lifecycle request, find that marketplace
     resource in inventory first and state second, then resolve that selected
     marketplace source into `ResolvedSourceCheckout`. The plugin operation
     does not invent a plugin-subdirectory source; the checkout source is the
     authoritative provenance.
   - For `MarketplaceRemove` and `PluginRemove`, build
     `ManagedProjectionInput::Remove` and perform no source resolution. Plugin
     removal keeps the existing unowned / missing-manifest guards; marketplace
     removal no longer emits `managed_project_source_missing` when the source
     is absent but the projected catalog surface is observable.
4. **Build one `ManagedProjectionContext`.** Construct `NativeLifecycleRequest`
   from the lifecycle spec, convert `NativeLifecycleKind` to
   `ManagedLifecycleKind`, and pass `target`, `project`, `paths`,
   `resource_key`, `resource_kind`, the request, `input`, `prior`,
   `acknowledged`, `filesystem`, and `json_limits` to the port. The
   `ResolvedSourceCheckout` is owned by the orchestrator local variable and
   borrowed only for the duration of `port.plan`; seed provenance and revision
   are cloned from it after planning.
5. **Call `port.plan`.** Map `ManagedProjectionError::{code, summary}` through
   the existing `managed_project_error` helper. Do not call any split
   `acquire`/`project` methods; they no longer exist.
6. **Consume returned evidence directly.** Sort/dedup `plan.manifest` and use
   `plan.current_fingerprint` and `plan.desired_fingerprint` as returned.
   Do not reconstruct MCP entries, omitted entries, current fingerprints, or
   desired fingerprints from `plan.files`, `plan.trees`, or target-native
   document bytes.
7. **Defense-in-depth acknowledgment.** If `acknowledged == false` and the
   returned manifest contains any `ManagedProjection::Omitted`, block with the
   existing partial-operation acknowledgment error before producing a planned
   operation. Required-unsupported components still surface as
   `ManagedProjectionError::RequiredUnsupported`, never as an omitted manifest
   entry.
8. **Validate ownership by target.** Call
   `validate_managed_project_ownership(kind, existing_state,
   plan.current_fingerprint.as_ref(), plan.desired_fingerprint.as_ref(),
   &managed_projections, installed_revision.as_ref(), &operation_id, target)`.
   The function body is unchanged except every target-state lookup uses the
   supplied `target` rather than a hardcoded Codex id.
9. **Build operation, entry, and seed.** Translate returned `files`/`trees` to
   the existing `ManagedProjectLifecycleEntry`, derive operation surfaces from
   those translated writes, and call the existing managed-materialization
   operation builder with the resolved target. For non-removal, seed
   `TargetResourceState` with target-local `Ownership::Skilltap`,
   `Provenance::Materialized`, source/revision from the checkout or prior
   target state, `desired_fingerprint`, and the returned manifest. For removal,
   seed stays `None`.
10. **Leave shared execution unchanged.** Existing state writes, pending-attempt
    recovery, retry behavior, publication, rollback, load verification, and
    foreground result rendering remain on the current shared path.

## Implementation notes

- The target-specific work in this story is limited to the adapter resolved by
  `registry.adapter(target)`. The CLI must not match on a target id string to
  select a codec, path, manifest shape, or projection fingerprint.
- `ManagedProjectPlanContext` should not add a `SourceRevisionResolver`; the
  amended contract moved checked-out source ownership to the orchestrator and
  kept Git checkout reuse in the existing `resolve_git_skill_source` helper.
- Remove the stale helpers implied by the old design (`observe_current_
  projection_fingerprint`, `plan_as_mcp`, CLI-side `managed_projection_
  manifest` reconstruction) from the Unit 3 implementation path. The adapter
  plan already carries complete evidence.
- Keep `lifecycle_operation_id` unchanged; it already takes `target:
  &HarnessId`, and operation ids remain stable for Codex because the lifecycle
  kind labels are unchanged.
- Error summaries should be target-neutral unless they come from an adapter's
  typed `ManagedProjectionError` detail. Do not retain Codex-specific text in
  shared orchestrator errors such as operation construction or state seeding.
- `3.0.0` is in quality gate. The state shape is unchanged
  (`STATE_SCHEMA_VERSION` stays), so this change must not be cherry-picked
  onto the release branch.

## Acceptance criteria

- [ ] `plan_managed_codex_project_lifecycle` and
      `ManagedCodexProjectPlanContext` no longer exist.
- [ ] `crates/cli/src/application/lifecycle.rs` dispatch around line 520 calls
      `plan_managed_project_lifecycle` through `adapter.managed_projection()`;
      the existing managed-project scope gate remains unchanged.
- [ ] `git grep -n 'HarnessId::new("codex")' crates/cli/` returns no matches.
- [ ] `git grep -n 'CodexManagedProjection' crates/cli/` returns no matches;
      CLI dispatch reaches Codex only through the registry-selected port.
- [ ] `git grep -n 'plan_as_mcp\|AcquiredProjection\|ManagedAcquisitionContext\|plan\.omitted' crates/cli/`
      returns no matches.
- [ ] `validate_managed_project_ownership` takes `target: &HarnessId` and
      preserves drift, unowned, update-required, and pending-attempt-recovery
      semantics identically for Codex.
- [ ] The orchestrator resolves `ResolvedSourceCheckout` only for apply
      lifecycles; both marketplace and plugin removal reach
      `ManagedProjectionInput::Remove` without source resolution.
- [ ] Plugin install/update resolves the selected marketplace source from
      inventory/state before planning and records that checkout source as the
      authoritative provenance; no target-specific CLI side channel supplies a
      second plugin source.
- [ ] The orchestrator persists `plan.manifest` and validates against
      `plan.current_fingerprint` / `plan.desired_fingerprint` directly, with
      no CLI reconstruction of target-native projection evidence.
- [ ] Defense-in-depth acknowledgment rejects any returned
      `ManagedProjection::Omitted` when `acknowledged == false`.
- [ ] Every existing Codex managed-project test passes with no assertion
      weakening beyond the already-approved source-free marketplace-removal
      behavior from the Codex adapter story.
- [ ] A temporary non-Codex fake port implements the current single
      `ManagedProjectionPort::plan` API, observes `Apply { checkout }` and
      `Remove`, returns a `ManagedProjectionPlan` with manifest/current/
      desired evidence, and drives a planned operation/entry/seed entirely
      through the port. Unit 4 will formalize this into the reusable matrix.
- [ ] `cargo test --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --all -- --check`, and `git diff --check` pass.

## Out of scope

- The shared acceptance matrix and formal fake-adapter profile (Unit 4 /
  `feature-managed-fallback-target-parity-acceptance`). This story keeps only
  the minimal proof needed to protect the dispatch flip.
- Any concrete adapter for a new target.
- Claude managed-project lifecycle changes.
