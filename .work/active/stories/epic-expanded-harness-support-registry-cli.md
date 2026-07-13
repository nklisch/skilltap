---
id: epic-expanded-harness-support-registry-cli
kind: story
stage: review
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
