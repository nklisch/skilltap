---
id: epic-expanded-harness-support-registry-cli
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry
  - epic-expanded-harness-support-registry-adapters
  - epic-expanded-harness-support-registry-config
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
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
  (`crates/cli/src/application/{status,reconciliation,lifecycle,instructions,\nexecution}.rs`) (modified):
  - `enabled_harnesses(config)` becomes `config.harnesses().enabled()`.
  - `instruction_locations`, `skill_destination`, `configured_native_profile`,
    `lifecycle_preview_presence`, and the lifecycle `HarnessKind` mapping
    dispatch through `registry.adapter(&id)` and the relevant adapter port.
  - Detection diagnostic and next-action messages reference
    `<registered-harness>` rather than `<codex|claude>`.

## Implementation notes

- The dispatch layer is the single point that holds a `&TargetRegistry`; it is
  threaded into the application services that previously matched on id strings.
- `--target all` already expands via the generic `resolve_targets`; the only
  change is that `enabled` now comes from the config map.
- Help derivation uses `Command::mut_arg` to set the `--target` help text from
  the registry; exact rendered text is verified by one assertion (a registered
  id appears in `--help`), not by maintained snapshots.
- `bootstrap`'s narrow Codex/Claude surface is preserved by filtering
  `first_party_targets()`; no other id becomes bootstrap-eligible.

## Acceptance criteria

- [ ] `skilltap --help` lists registered harnesses with no hardcoded id string
      in the rendering path.
- [ ] `skilltap harness enable gemini` (not yet registered) fails with
      `target_not_registered` at the composition boundary and writes nothing.
- [ ] `skilltap harness enable codex` and `... claude` behave exactly as today
      (existing compiled-binary tests pass unchanged).
- [ ] `skilltap bootstrap --target codex` and `... claude` succeed; any other
      id is rejected because it is not a `FirstPartyPlugin` target.
- [ ] `--target all` expands to every enabled registered harness from the map.
- [ ] No behavior-dispatching `match target.as_str()` remains in
      `crates/cli/src/`.

## Out of scope

- The test-support acceptance contract (Unit 5).
- Any new target adapter.
