---
id: epic-expanded-harness-support-registry-cli
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
  - epic-expanded-harness-support-registry-adapters
  - epic-expanded-harness-support-registry-config
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-13
---

# CLI Parser, Help, and Composition Dispatch

## Scope

Implement Unit 4 of the registry feature design. Wire the `TargetRegistry` into
the CLI so that `--target`/positional help, target membership validation, and
the per-target composition sites in `crates/cli/src/application.rs` (and its
submodules) derive from the registry and from the registry-driven config map
rather than from Codex/Claude string matches.

## Units

- `crates/cli/src/command.rs` (modified):
  - `parse_harness`/`parse_target` parse any structurally valid `HarnessId`;
    drop the hardcoded `"codex"|"claude"` literals. Registry membership is
    enforced in dispatch.
- `crates/cli/src/entrypoint.rs` (modified):
  - Build `TargetRegistry::canonical()` once in the composition root.
  - Augment clap `--target` / harness-positional help from `registry.ids()` so
    `skilltap --help` enumerates registered harnesses without literals.
  - Add membership validation that emits `target_not_registered` for ids not in
    the registry, before any state write.
  - Restrict `bootstrap --target` to `registry.first_party_targets()`.
- `crates/cli/src/application.rs` and submodules
  (`crates/cli/src/application/{status,reconciliation,lifecycle,instructions,execution}.rs`) (modified):
  - `enabled_harnesses(config)` becomes `config.harnesses().enabled()`.
  - `instruction_locations`, `skill_destination`, `configured_native_profile`,
    `lifecycle_preview_presence`, and lifecycle dispatch use
    `registry.adapter(&id)` and the relevant adapter port.
  - Detection diagnostic and next-action messages reference
    `<registered-harness>` rather than `<codex|claude>`.
- Final compatibility-seam removal across `crates/harnesses/` (modified):
  - migrate `bootstrap.rs`, `lib.rs`, and `lifecycle.rs` from `HarnessKind` to
    `HarnessId`/registry adapter dispatch;
  - drop `NativeLifecycleRequest.harness` after every CLI caller selects the
    adapter before constructing the request;
  - remove compatibility wrappers that accept `HarnessKind`;
  - migrate `crates/harnesses/tests/{bootstrap,detection,lifecycle_scope}.rs`
    to registry adapters and typed harness ids.

## Implementation notes

- The dispatch layer is the single point that holds a `&TargetRegistry`; it is
  threaded into the application services that previously matched on id strings.
- This story owns the final `HarnessKind` compatibility-seam removal across both
  CLI consumers and harnesses-crate producers/tests. The adapter story keeps the
  seam only so its intermediate commit remains compilable; this integration
  story must leave `git grep -n "HarnessKind" crates/` empty.
- `--target all` already expands via the generic `resolve_targets`; the only
  change is that `enabled` now comes from the config map.
- Help derivation uses `Command::mut_arg` to set the `--target` help text from
  the registry; exact rendered text is verified by one assertion (a registered
  id appears in `--help`), not by maintained snapshots.
- `bootstrap`'s narrow Codex/Claude surface is preserved by filtering
  `first_party_targets()`; no other id becomes bootstrap-eligible.

## Implementation record

- Execution capability: highest, as directed by the active autopilot caller
  because this is the cross-crate composition and native-contract migration.
- Review weight: standard (caller).
- Dispatch: direct implementation only, as required; no delegation or peer
  mechanism was used.
- Built one canonical `TargetRegistry` in `run_from`, derived root/leaf target
  help recursively from `registry.ids()`, and validated every explicit target
  before repository composition. Unregistered ids now return stable
  `target_not_registered` with command-help guidance; the compiled no-write
  regression proves `harness enable gemini` does not create configuration.
- Threaded the registry through `StatusApplication`. Enabled selection comes
  from `HarnessPolicyMap::enabled`; config documents containing an unregistered
  key fail at the composition boundary before mutation. Detection, capability
  selection, bounded status observation, instruction bridges, skill
  destinations, lifecycle preview/execution, managed-project fallback, harness
  list roots, and first-party bootstrap now dispatch through adapter ports or
  adapter metadata.
- Removed `HarnessKind`, its detection/profile compatibility wrappers, and
  `NativeLifecycleRequest.harness`. Added `NativeLifecycleDispatch` to bind a
  semantic request to the already-selected `HarnessId` and
  `NativeLifecycleVector`; `NativeLifecyclePort` revalidates that binding against
  each planned operation under lock.
- Extended adapter metadata narrowly with native root, managed-project fallback,
  first-party bootstrap actions, alternate project instruction bridges, and
  authored status-surface labels. These preserve target-specific behavior in
  adapter-private code instead of introducing another target list in CLI.
- Preserved Codex's interactive bootstrap gap and exact next action, Claude's
  capability fallback action, native argv, user/local scope evidence, lifecycle
  error precedence, bridge preservation/consolidation, skill projection paths,
  first-use reporting, diagnostics, and observation limits. Missing optional
  project roots now leave `project_entry_count` absent while real observation
  limit failures still propagate.
- Test migration: harness detection/bootstrap/lifecycle-scope tests now use
  concrete registry adapters and typed lifecycle dispatch. CLI parser tests pin
  structural parsing separately from composition membership; entrypoint and
  compiled-binary tests pin registry-derived help and no-write rejection.
- Test-fixture discrepancy: profile executables published cross-device as
  symlinks are canonicalized by production executable resolution, which makes a
  sibling behavior file unreachable. Compiled CLI tests now use the existing
  ordinary `install_alias` API in per-target isolated directories; production
  resolution and test-support public APIs remain unchanged.
- Simplification: removed the closed enum, request target duplication, detection
  and profile wrappers, CLI target string matches, hardcoded enabled list, and
  duplicated status observation routing.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core` — 345 passed across 6 suites.
- `cargo test -p skilltap-harnesses` — 56 passed across 6 suites.
- `cargo test -p skilltap-test-support` — 20 passed across 2 suites.
- `cargo test -p skilltap --all-targets` — 136 passed across 6 suites.
- `cargo test --workspace --all-targets` — 558 passed across 18 suites.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo check --workspace --all-targets` — passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.
- `git grep -n HarnessKind -- crates` — no matches.
- No target-id behavior match remains in `crates/cli/src`; remaining
  Codex/Claude literals are adapter-private contracts, managed Codex native
  projection code, display/diagnostic text, and tests.

## Acceptance criteria

- [x] `skilltap --help` lists registered harnesses with no hardcoded id string
      in the rendering path.
- [x] `skilltap harness enable gemini` (not yet registered) fails with
      `target_not_registered` at the composition boundary and writes nothing.
- [x] `skilltap harness enable codex` and `... claude` behave exactly as today
      (existing compiled-binary tests pass unchanged apart from assertions that
      now expect composition-layer membership diagnostics).
- [x] `skilltap bootstrap --target codex` and `... claude` remain eligible; any
      other id is rejected because it is not a `FirstPartyPlugin` target.
- [x] `--target all` expands to every enabled registered harness from the map.
- [x] No behavior-dispatching `match target.as_str()` remains in
      `crates/cli/src/`.
- [x] `git grep -n "HarnessKind" crates/` returns no matches after CLI and
      harnesses producer/test migration.
- [x] `NativeLifecycleRequest` no longer carries a `HarnessKind` field; callers
      select `NativeLifecycleVector` from `TargetRegistry` before request
      construction.

## Out of scope

- The test-support acceptance contract (Unit 5).
- Any new target adapter.

## Review (standard, cross-model GLM-over-OpenAI fresh context)

Cross-model review of an OpenAI-host implementation by GLM 5.2. Verified
end-to-end at commit `2b2ffc17` against the story body, parent feature
design, sibling child reviews, foundation docs, project rules, the
`patterns` skill, and the actual diff. Reran focused and full checks.

Verified claims:

- One authoritative registry: `TargetRegistry::canonical()` is the sole
  production constructor, built once in `entrypoint::run_from` and threaded
  through `StatusApplication`. No second registry exists in CLI.
- CLI parsing vs composition membership: `parse_harness`/`parse_target`
  (`command.rs`) only do structural validation; `Dispatch::validate_targets`
  enforces registry membership and emits `target_not_registered` before any
  state write. `Dispatch::harness_argument` covers `harness enable|disable`.
- Recursive help augmentation: `augment_target_help` walks `--target`, `--from`,
  and the harness positional across the root and every subcommand; the
  `Registered harnesses: codex|claude` after-help is asserted in entrypoint
  tests.
- Pre-write unknown-target rejection: `validate_targets` runs in `run_from`
  before dispatch; `execute_harness_change` and `load_documents` re-check
  `config_membership_error` so even a config file containing an unregistered
  key fails closed at the composition boundary. The compiled
  `unregistered_harness_is_rejected_before_state_creation` test confirms
  `harness enable gemini` writes no configuration.
- enabled/all semantics: `enabled_harnesses` derives from
  `HarnessPolicyMap::enabled()`; `--target all` still expands through the
  generic `resolve_targets`.
- First-use status: `first_use_harness_report` iterates `registry.iter()`
  filtered by the requested selection, dispatching detection through
  `detect_configured_installation`.
- Detection and strict observation: `NativeObservation::run` and
  `StatusProjection::apply` resolve every adapter through
  `registry.adapter(target)`; capability, surface, and finding rendering
  stay registry-driven.
- Instruction bridge set including alternates: `InstructionBridgePort`
  exposes `global_bridge`, `project_bridge`, and `alternate_project_bridges`;
  the Claude `.claude/CLAUDE.md` alternate is observed, preserved, and
  consolidated via adapter metadata, with no CLI-side Claude literal.
- Skill destination: `SkillProjectionPort::destination` drives
  `skill_destination`; canonical `~/.agents/skills` projection is preserved
  alongside per-target destinations.
- Lifecycle request/dispatch/revalidation under lock: `NativeLifecyclePort`
  stores `NativeLifecycleDispatch` entries by `OperationId` and
  revalidates `scope`, `action`, and `target` against each planned
  operation in `ExecutionPort::revalidate` under `SystemConfigurationLock`.
  NoOp operations re-observe fresh native evidence before being honored.
- Native argv and scope: `NativeLifecycleVector::arguments` is fully owned
  by each adapter; Codex precedence (PluginUpdate before project scope) is
  regression-tested, and `observation_scope` carries Claude's user/local
  scope evidence.
- Status labels/subjects: `AdapterObservationPaths::surface_labels` are
  adapter-authored; the CLI never reinterprets target-specific paths.
- Bootstrap eligibility/actions: `bootstrap_commands` iterates
  `registry.first_party_targets()`; `Dispatch::validate_targets` rejects
  non-first-party registered targets with `bootstrap_target_unavailable`.
  Codex's `bootstrap_next_action` preserves the interactive gap; Claude
  keeps the capability-fallback path.
- Managed Codex fallback: `CodexAdapter::managed_project_lifecycle` returns
  true and `application/lifecycle.rs` routes project plugin/marketplace
  work through `plan_managed_codex_project_lifecycle` when the selected
  adapter opts in. Codex literals there are managed-fallback contracts,
  not behavior dispatch.
- No behavior-dispatch lists outside adapter metadata: `git grep` confirms
  no `match target.as_str()` / `match harness.as_str()` in `crates/cli/src`;
  remaining Codex/Claude literals are display text, managed Codex
  projection contracts, or wire-compat ordering in `HarnessPolicyMap`.

Verified removals:

- `git grep -n HarnessKind -- crates` returns no matches.
- `NativeLifecycleRequest` carries only `action`, `scope`, `name`, `source`;
  `NativeLifecycleDispatch` binds the semantic request to the
  already-selected `HarnessId` and `NativeLifecycleVector`.

Contract extensions inspected (all adapter-private, no new CLI dispatch):

- `HarnessAdapter::decode_version_with_limits` (defaulted), `native_root`,
  `managed_project_lifecycle`, `bootstrap_next_action`,
  `bootstrap_capability_next_action`.
- `NativeLifecycleVector::observation_scope`.
- `InstructionBridgePort::alternate_project_bridges`.
- `AdapterObservationPaths::surface_labels`.
- `NativeLifecycleDispatch` (target binding) and the `NativeLifecyclePort`
  `with_foreign_operations` seam for mixed native/managed plans.

Error/exit/plain/JSON compatibility: `entrypoint/tests.rs` pins the
`target_not_registered` plain/JSON split, the missing-subcommand usage
fallback, and the stdout/stderr channel rules; compiled-binary tests pin
the same codes at the process boundary.

Compiled-test `install_alias` workaround: `FakeNativeProcess::install_alias`
uses `fs::copy` for both the executable and the sibling behavior file so
the production canonicalizing resolver still finds the behavior file at
the destination. This accommodates the existing symlink-canonicalization
security property of `SystemExecutableResolver` (which is unchanged by
this story) and does not mask any production executable-resolution
defect; production resolution and the test-support public API are
unchanged.

Re-run verification at `2b2ffc17`:

- `cargo test --workspace --all-targets` — 558 passed across 18 suites.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo fmt --all -- --check` and `git diff --check` — clean.
- `git grep -n HarnessKind -- crates` — no matches.

Non-blocking observations (parked, not blockers):

- `bootstrap_target_unavailable` has no direct test because every currently
  registered target is `FirstPartyPlugin`; the path is structurally covered
  by the registry `first_party_targets` tests and will gain coverage when a
  `Managed` adapter lands.
- `native_surface_kind` in `application/status.rs` classifies the
  `.claude` project root as `Plugin` via `root.ends_with("claude")`. This
  is presentation-layer ResourceKind inference from adapter-authored root
  names, not behavior dispatch, so it does not violate the no-dispatch
  contract; an adapter-declared kind mapping would be cleaner if more
  targets grow non-suffix-derivable roots.

Verdict: approve. All acceptance criteria met; Codex/Claude behavior,
wire compatibility, error/exit contracts, and the no-write-on-reject
invariant are preserved; `HarnessKind` and request-target duplication are
fully removed.
