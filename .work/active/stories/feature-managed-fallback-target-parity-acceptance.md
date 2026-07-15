---
id: feature-managed-fallback-target-parity-acceptance
kind: story
stage: done
tags: []
parent: feature-managed-fallback-target-parity
depends_on: [feature-managed-fallback-target-parity-contract, feature-managed-fallback-target-parity-codex-adapter, feature-managed-fallback-target-parity-orchestrator]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-13
updated: 2026-07-15
---

# Shared Managed-Projection Acceptance Matrix

## Scope

Implement Unit 4 of the managed-fallback-target-parity feature design: a
reusable `managed_acceptance_matrix` in `skilltap-test-support` that every
`ManagedProjectionPort` adapter must pass, mirroring the registry's
`acceptance_matrix` for native lifecycle. Port the existing Codex
managed-project tests onto the matrix via `ManagedProjectionProfile::codex()`
without assertion changes, and add a fake-adapter profile that proves the
orchestrator is target-agnostic through the current single-method
`ManagedProjectionPort::plan` API.

This story is the reusable contract every sibling adapter feature
(file-managed, native-coexistence, configuration-constrained,
trust-interactive, Pi) will invoke with its own profile.

Parent design: `feature-managed-fallback-target-parity` Unit 4.

## Units

- `crates/test-support/src/managed_acceptance.rs` (new):
  `managed_acceptance_matrix`, `ManagedProjectionProfile`,
  `ManagedAcceptanceReport`. Re-export from
  `crates/test-support/src/lib.rs`.
- `crates/test-support/src/harness_profile.rs` (modified):
  `FakeHarnessProfile` gains an optional
  `managed_projection: Option<ManagedProjectionProfile>` so adapters that
  opt into managed fallback get the full matrix; adapters that do not are
  skipped.
- `crates/cli/src/application/tests.rs` (modified): the existing Codex
  managed-project tests are ported onto
  `ManagedProjectionProfile::codex()` without assertion changes; the
  temporary fake-adapter proof from Unit 3 is formalized as a
  `ManagedProjectionProfile` for a non-Codex `HarnessId` whose port observes
  `ManagedProjectionInput::Apply { checkout }` and `Remove`, and returns a
  `ManagedProjectionPlan` carrying `files`, `trees`, `manifest`,
  `current_fingerprint`, and `desired_fingerprint`.

The matrix covers (per the parent design):

- Marketplace acquisition (catalog read at `.agents/plugins/marketplace.json`
  and `.claude-plugin/marketplace.json`).
- Plugin acquisition (marketplace source → catalog → plugin tree).
- Complete skill-tree projection (top-level `SKILL.md` preserved; never
  reduced to a single file).
- MCP merge into the adapter-owned document format (Codex TOML
  `mcp_servers` at `.codex/config.toml`).
- Foreground acknowledgment of optional omissions (plugin-root-relative MCP
  executable → `ManagedProjection::Omitted { consequence:
  plugin_root_relative_mcp_omitted }` only with `--yes`).
- Required-unsupported blocking (blocks even with `--yes`).
- Drift detection, unowned-destination rejection, update-required rejection.
- Pending-attempt recovery (install/update with publication failure → retry
  → noop).
- Effective-load verification via the existing `LoadVerifier` (fresh
  observation; cache inspection is not verification).
- Immediate-repeat idempotency (second pass → `OperationOutcome::NoChange`,
  no duplicate artifacts or state entries).

## Implementation notes

- The Codex instance of the matrix reuses the existing tests at
  `crates/cli/src/application/tests.rs:582` (publication failure retry +
  noop), `:725` (tree-limit revalidation), `:833-969` (pending-attempt
  recovery for install/update), and `:1360-1506` (ownership validation).
  Those tests are ported onto the matrix's `ManagedProjectionProfile::codex()`
  without assertion changes.
- The fake-adapter profile (the formalization of Unit 3's temporary proof)
  registers a throwaway `ManagedProjectionPort` for a non-Codex `HarnessId`
  and asserts the orchestrator resolves the apply checkout, passes removal
  with no checkout, consumes the returned manifest/current/desired
  fingerprint evidence directly, and drives ownership, drift, and idempotency
  through the port. This is the canary for abstraction leakage: if it cannot
  exercise the full matrix through a non-Codex profile without a target-
  specific CLI side channel, the port has leaked Codex shape.
- `FakeHarnessProfile::codex().managed_projection` is `Some`; Claude's
  profile reflects Claude's managed-fallback opt-in state (preserved as-is).
- Low-value tests are not added: no per-field serialization test for
  `ResolvedSourceCheckout` or `ManagedProjectionPlan` (they are planning
  currency, not serialized), no exhaustive `ManagedProjectionError` code table
  beyond what the orchestrator surfacing exercises, no snapshot of MCP TOML
  bytes (the Codex regression already pins the format), and no separate test
  of the `From` conversions beyond the orchestrator integration.

## Acceptance criteria

- [x] `managed_acceptance_matrix(&ManagedProjectionProfile::codex(), runner)`
      passes the full acquisition/projection/MCP/acknowledgment/drift/unowned/
      update-required/pending-recovery/verification/idempotency suite, with the
      existing Codex publication, tree-limit, pending-attempt, and removal
      assertions moved under the matrix without weakening.
- [x] A fake-adapter `ManagedProjectionProfile` for a non-Codex `HarnessId`
      passes the same matrix through `ManagedProjectionPort::plan`, proving
      the orchestrator is target-agnostic, `Apply` receives exactly one
      `ResolvedSourceCheckout`, `Remove` receives no checkout, and the port
      does not leak Codex shape.
- [x] `FakeHarnessProfile::codex().managed_projection` is `Some`; Claude's
      matches its managed-fallback opt-in.
- [x] Immediate-repeat idempotency holds: running the matrix twice produces
      `OperationOutcome::NoChange` on the second pass with no duplicate
      artifacts or state entries.
- [x] `cargo test --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --all -- --check`, and `git diff --check` pass.

## Out of scope

- Any concrete adapter for a new target (sibling adapter features supply
  their own `ManagedProjectionProfile`).
- Claude managed-project lifecycle changes.
- Changes to the publication boundary (`PublicationBatch`/
  `PublicationSink`/`LoadVerifier`) — it is consumed as-is.

## Implementation completion notes

- Execution capability: highest, inherited from the active autopilot run because this acceptance contract protects shared adapter dispatch, persisted ownership evidence, rollback, and retry semantics.
- Review weight: standard (project/autopilot default).
- Dispatch: inline only, as required by the caller; no subagent or peeragent was used.
- Files changed: added `crates/test-support/src/managed_acceptance.rs`; updated test-support re-exports and `FakeHarnessProfile`; expanded `crates/cli/src/application/tests.rs` with the dependency-aware runners and production lifecycle assertions.
- Reusable boundary: test-support owns dependency-neutral `ManagedProjectionProfile`, `ManagedAcceptanceScenario`, `ManagedAcceptanceCheck`, evidence, report, and completeness validation. The CLI runner translates those descriptors into validated production types and actual lifecycle dispatch, avoiding a test-support → core/harnesses/CLI package cycle.
- Codex regression migration: the existing publication-failure, source-free-removal, tree-limit, and terminal-journal functions are now matrix scenarios; their unique assertions were retained. The matrix additionally pins both accepted catalog source locations, complete three-file skill projection, MCP merge preservation, manifest/fingerprint evidence, omission acknowledgment, required-only blocking, target-local sibling preservation, drift/unowned/update-required rejection, and duplicate-free repeats.
- Non-Codex proof: the fake adapter handles marketplace and plugin resources through the single `ManagedProjectionPort::plan` method, returns complete tree/file writes plus manifest/current/desired evidence, counts one plan call per apply checkout, and proves source-free removal through the shared production lifecycle.
- Fresh verification discrepancy: the current managed lifecycle does not call core's `LoadVerifier`; `ManagedProjectLifecyclePort` verifies publication through fresh post-write file/tree reads. The matrix injects a post-write read failure and verifies rollback plus successful retry at that real boundary rather than claiming coverage of an unused abstraction.
- Simplification: four formerly standalone Codex regression functions now run under one named matrix scenario set instead of duplicating their assertions; future adapters add a profile and production-aware runner without changing the matrix vocabulary.
- Verification: `cargo test --workspace --all-targets` (562 passed), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` passed. The focused two-profile matrix also passed ten consecutive commands; each command executes the matrix twice.
- Discrepancies from design: the proposed profile could not store `HarnessId` or `&dyn ManagedProjectionPort`, and the proposed matrix could not directly own an `IsolatedMachine` production runner, because `skilltap-test-support` is a dev dependency of the production crates. The dependency-neutral callback/evidence API is the nearest dependency-correct form and preserves the intended reusable contract.
- Adjacent issues parked: none.

## Review (standard, approved at 7f74ee12)

Verdict: approved. This is trustworthy, reusable coverage for sibling adapters.
The matrix enforces one complete scenario set; both profiles earn every
required check through real production lifecycle dispatch with substantive
assertions, not self-reported labels.

### Verification performed

- `cargo test --workspace --all-targets`: 562 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check` and `git diff --check`: clean.
- Focused matrix test run 5x consecutively: stable, no flake.
- Diff scope confirmed: only this story, `tests.rs`, `harness_profile.rs`,
  test-support `lib.rs`, and the new `managed_acceptance.rs`. No production
  code, parent feature, downstream stories, contract story, or `.pi/`.

### Evidence is earned, not self-reported

The matrix function only verifies the runner returns the required evidence
labels per scenario; the substance lives in the CLI runner. The pattern is
sound because each `*_acceptance()` runner performs its concrete assertions
(`assert_eq!`/`assert!`/`assert_error_code`/state-shape matches) BEFORE
returning `evidence([...])`. A failing assertion panics before any label is
returned, so labels cannot be claimed without the underlying check passing.
The matrix's own self-tests (`matrix_requires_each_scenarios_declared_evidence`,
`complete_matrix_reports_every_scenario_and_check`) prove the completeness
enforcement itself.

Each required scenario is covered with real dispatch for BOTH profiles:

- Pending retry/noop: fake injects atomic-write failure then retries with
  pending-attempt recovery and stable publication count; Codex runs the
  byte-identical `managed_project_publication_failures_restore_then_retry_once_and_noop`
  and terminal-journal recovery functions.
- Drift/unowned/update-required: both profiles mutate the destination or
  source and assert `managed_project_drifted`, `managed_project_unowned`,
  and `managed_project_update_required`.
- Acknowledgment/required-unsupported: both profiles block without `--yes`,
  accept optional omission with `--yes`, and block required failures even
  with `--yes`.
- Source-free removal: both profiles remove after deleting the source and
  assert files are gone; Codex also invokes the byte-identical
  `managed_marketplace_removal_uses_observed_projection_without_source`.
- Target isolation: both profiles insert a sibling target state and assert
  the repeat install preserves it.
- Complete tree/MCP evidence: fake asserts three skill files; Codex asserts
  both catalog destinations, three skill files, and MCP merge preservation.
- Idempotence/no duplicates: both profiles snapshot the project tree,
  publication count, resource count, and manifest length across a repeat.
- Fresh load: both inject a post-write read failure and assert rollback +
  successful retry.

### No hidden weakening or arbitrary bundling

The four previously-standalone Codex regression functions
(`managed_project_publication_failures_restore_then_retry_once_and_noop`,
`managed_marketplace_removal_uses_observed_projection_without_source`,
`managed_project_tree_limits_preserve_planning_and_revalidation_failures`,
`managed_terminal_journal_failure_recovers_without_duplicate_projection_publication`)
are byte-identical to their pre-commit bodies; only their `#[test]` attributes
were removed and they are now invoked from inside the Codex matrix runner.
No assertion was weakened or deleted. The orchestrator story's temporary
proof (`non_codex_managed_port_drives_apply_acknowledgment_evidence_and_source_free_remove`)
was folded into the fake-adapter scenarios, which expand its assertions across
the full scenario set rather than collapsing them.

### Documented deviations validated as non-material

- **Dependency-neutral callback shape.** `skilltap-test-support` is a
  `[dev-dependencies]` of all three production crates and has zero
dependencies of its own, so it cannot import `HarnessId`, `IsolatedMachine`,
  or `&dyn ManagedProjectionPort` without forming a cycle. The
  callback/evidence API (`ManagedProjectionProfile` with `&'static str`
  fields, runner closure) is the nearest dependency-correct form. The design's
  Unit 4 sketch (`id: HarnessId`, `port: fn() -> ...`, `&IsolatedMachine`) was
  not realizable; the implemented shape preserves the reusable contract.
- **`LoadVerifier` not used by the managed lifecycle.** Confirmed against
  `crates/cli/src/application/execution.rs`: the `ManagedProjectLifecyclePort::apply`
  path verifies each write by reading it back (`read_regular_bounded_no_follow`,
  lines 391 and 482) and rolls back on mismatch with `Managed project file
  verification failed.` / `Managed project skill verification failed.` That
  is the substantive load-verification guarantee (fresh post-write
  observation, not assumption). The matrix's `fail_next_post_write_read`
  injection hits exactly that boundary, proving rollback + successful retry.
  The named `LoadVerifier` abstraction is not exercised, but the guarantee it
  represents is. The deviation is transparently documented and accurate.

### Code economy and parameterization

The ~1459-line expansion is justified: 8 scenarios x 2 profiles (16 runner
functions), the fake adapter's realistic plan/evidence production (required
to drive ownership/drift/idempotency through the real orchestrator), and
shared helpers. Each function has one clear purpose; no obvious bloat.
Parameterization introduces no shared false positives: the matrix requires
ALL declared evidence per scenario, evidence is only returned after
assertions pass, and the matrix self-tests prove enforcement.

### Notes (lower-risk, non-blocking)

- State-target cleanup after managed removal is no longer asserted
  explicitly (the old orchestrator proof asserted
  `state...target(...).is_none()` after `MarketplaceRemove`). The new matrix
  asserts file cleanup, source-free removal, completion, and (fake) the
  remove plan count; state cleanup is exercised implicitly through
  `assert_changed` and the shared executor that owns state removal. Park as
  a minor explicitness opportunity for a future adapter-profile pass; not a
  current-cycle blocker because removal is otherwise extensively covered and
  state cleanup is owned by the shared lifecycle executor.
- The Codex `RequiredUnsupported` matrix check is earned through
  `managed_project_plugin_invalid`/`managed_project_plugin_unsupported` (a
  missing required `SKILL.md` fails at catalog validation), not the typed
  `ManagedProjectionError::RequiredUnsupported` code. The behavioral
  guarantee (required-unprojectable blocks even with `--yes`) holds; the
  typed path is covered by the fake adapter. Naming is slightly imprecise
  for the Codex side but the contract is met.

### Outcome

Story advances to `done`. The reusable managed-projection acceptance contract
is trustworthy: every sibling adapter feature can add a `ManagedProjectionProfile`
and a production-aware runner and inherit the full scenario enforcement.
