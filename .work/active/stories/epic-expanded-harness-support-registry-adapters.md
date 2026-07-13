---
id: epic-expanded-harness-support-registry-adapters
kind: story
stage: review
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Codex and Claude Adapter Migration

## Scope

Implement Unit 2 of the registry feature design. Move the existing Codex and
Claude detection, version-decode, capability-profile, observation, native-
lifecycle-vector, instruction-bridge, and skill-projection logic out of the
closed `HarnessKind` match sites and onto `CodexAdapter` / `ClaudeAdapter`
structs that implement the `HarnessAdapter` trait defined in the parent
feature. Register both in `TargetRegistry::canonical()`.

This story proves the contract is concrete and that Codex/Claude behavior is
preserved byte-for-byte. It does not implement any new target adapter.

## Units

- `crates/harnesses/src/adapters/mod.rs` (new): adapter module root and any
  shared `adapter_helpers` relocation target.
- `crates/harnesses/src/adapters/codex.rs` (new): `CodexAdapter` plus
  `CodexLifecycle`, `CodexInstructionBridge`, `CodexSkillProjection`.
- `crates/harnesses/src/adapters/claude.rs` (new): `ClaudeAdapter` plus the
  corresponding Claude port structs.
- `crates/harnesses/src/lib.rs` (modified): relocate the existing
  `observe_codex_*` / `observe_claude_*` / `decode_native_version` /
  `select_profile` / `compiled_capabilities` / `unknown_capabilities` free
  functions behind a private `adapter_helpers` module; re-export the registry
  and adapter modules; remove the `HarnessKind` enum after all call sites are
  converted.
- `crates/harnesses/src/lifecycle.rs` (modified): drop `harness: HarnessKind`
  from `NativeLifecycleRequest`; the owning adapter's `NativeLifecycleVector`
  supplies the per-harness argument vector, including the Codex
  project-scope-unsupported constraint.

## Implementation notes

- Relocate, do not rewrite: the adapter structs delegate to the existing
  functions unchanged so behavior is preserved. The capability matrix (Codex
  `0.144.1` with no plugin.update and no project-scope marketplace/plugin
  lifecycle; Claude `2.1.201` with the asymmetric update/project matrix) is
  reproduced exactly.
- `CodexAdapter` and `ClaudeAdapter` are stateless singletons exposing
  `static_ref() -> &'static dyn HarnessAdapter`, satisfying the registry's
  `&'static` storage.
- After migration, `git grep -n "HarnessKind" crates/` must return no matches.

## Acceptance criteria

- [ ] Every existing Codex and Claude detection, capability, observation,
      lifecycle, instruction, and skill-projection test passes without
      modification to its assertions.
- [ ] A capability-matrix table test asserts `CodexAdapter::select_profile`
      and `ClaudeAdapter::select_profile` reproduce today's `select_profile`
      output for the verified version and for an unknown version.
- [ ] `git grep -n "HarnessKind" crates/` returns no matches.
- [ ] `git grep -n '"codex"\|"claude"' crates/cli/src/` returns no behavior-
      dispatching match arms (display labels only, if any).
- [ ] `TargetRegistry::canonical().ids()` yields exactly `codex` and `claude`.

## Out of scope

- The CLI parser/help/config changes (Unit 3/4) and the test-support contract
  (Unit 5) land in their own stories.
- Any new target adapter.

## Implementation notes

Implemented concrete stateless `CodexAdapter` and `ClaudeAdapter` singletons and
registered them, in stable order, in `TargetRegistry::canonical()`. Detection,
version decoding, exact-version capability selection, unknown-version
observe-only profiles, bounded canonical observation, native lifecycle vectors,
instruction bridges, and skill projection destinations now dispatch through the
adapter ports. Shared decoding, profile, path, and observation composition lives
in the private `adapter_helpers` module. Existing lifecycle execution remains a
direct argument-vector boundary; Codex project lifecycle and plugin-update
constraints now live on `CodexLifecycle`, while Claude's user/local scope mapping
lives on `ClaudeLifecycle`.

The exact capability table is pinned by an adapter matrix test for Codex
`0.144.1`, Claude `2.1.201`, and unknown versions. Existing detection,
observation, lifecycle, instruction-adjacent, and projection-adjacent harness
tests retain their assertions and pass.

### Compatibility seam

`HarnessKind` and the `harness` field on `NativeLifecycleRequest` remain as a
narrow public compatibility token because the out-of-scope CLI, bootstrap
rendering, and test-support/integration callers still construct and exhaustively
match that surface. The token no longer owns detection, profile, observation,
or lifecycle-vector behavior: it resolves its adapter through
`TargetRegistry::canonical()`.

Commit `3119913c` closes the review's ownership hole by extending the active
`epic-expanded-harness-support-registry-cli` story. Its durable Units and
acceptance criteria explicitly own final seam removal across CLI consumers,
`crates/harnesses/src/{bootstrap,lib,lifecycle}.rs`, and
`crates/harnesses/tests/{bootstrap,detection,lifecycle_scope}.rs`, including
removing `NativeLifecycleRequest.harness`, compatibility wrappers, and every
repository `HarnessKind` occurrence. Keeping the seam here therefore preserves
an independently compilable intermediate commit without leaving elimination
unowned.

This is an intentional sequencing seam rather than a new compatibility layer:
no additional target behavior may be added to `HarnessKind`.

### Approved contract extensions and deviations

This implementation necessarily extends the approved `registry.rs` trait shape
in two narrowly bounded ways:

- `HarnessAdapter::decode_version_with_limits` preserves the existing strict-JSON
  decode boundary's caller-supplied `JsonLimits`. The approved
  `decode_version(stdout)` signature could preserve text decoding but could not
  reproduce the prior bounded JSON behavior. Its default delegates to
  `decode_version`, so text-only adapters do not gain boilerplate and existing
  implementations remain source-compatible.
- `NativeLifecycleVector::observation_scope` moves native scope interpretation
  behind the owning lifecycle adapter. Claude list evidence must distinguish
  skilltap global/project scope as native `user`/`local`; without this port,
  generic postcondition observation would either reintroduce a Claude string or
  enum dispatch in `lifecycle.rs`, or incorrectly accept a same-name resource
  from the wrong scope. Codex returns `None` because its verified lifecycle has
  no independently encoded scope evidence.

These are approved contract extensions required to relocate existing boundary
semantics, not new product behavior. Apart from those interface additions and
the temporary compatibility seam, detection bytes and limits, capability
matrices, canonical observations, lifecycle argv and rejection diagnostics,
instruction paths, skill projection paths, and native scope interpretation
remain byte-equivalent to the pre-migration behavior.

### Verification

- `cargo test -p skilltap-harnesses` — 56 passed across six suites, including
  the Codex PluginUpdate+Project precedence regression.
- `cargo clippy -p skilltap-harnesses --all-targets -- -D warnings` — passed.
- `cargo fmt --package skilltap-harnesses -- --check` — passed.
- `cargo check --workspace` is temporarily blocked by the concurrent config-map
  migration: current CLI code still accesses removed `HarnessPolicyMap.codex`
  and `.claude` fields. That failure is wholly outside this story's ownership
  and is the expected Unit 3 → Unit 4 integration seam.

## Review (2026-07-12)

**Verdict**: Bounce
**Review weight**: standard, risk-escalated to Deep fresh-context because this
story relocates native detection / version-decode / capability-profile /
observation / lifecycle / instruction / skill-projection contracts onto the
new adapter trait.
**Reviewer context**: cross-model — Z.AI GLM 5.2 fresh-context review of an
OpenAI-host run (different model class).

### Blocker — the compatibility seam is not actually owned by any active story

The story's body retains `HarnessKind` (and the `NativeLifecycleRequest.harness`
field) as a "narrow public compatibility token" and asserts that "Those owning
stories must migrate callers to `HarnessId` plus registry adapter dispatch,
drop the compatibility wrappers/request field, and then satisfy the
repository-wide `git grep -n "HarnessKind" crates/` and CLI string-dispatch
checks." The argument depends on the already-active CLI and test-support
stories owning *complete* elimination. They do not.

Verified `git grep -n HarnessKind crates/` at this commit returns **83
non-CLI occurrences** plus the CLI surface. Of those, neither active story
explicitly owns the harnesses-crate production sites:

- `crates/harnesses/src/bootstrap.rs` — **19 production occurrences**, and
  they are behavior-dispatching, not display labels. Examples: `if target ==
  HarnessKind::Codex { return Unsupported { ... } }` (line 161),
  `unsupported_next_action(target: HarnessKind)` returning per-harness
  next-action strings (lines 292–296), `HarnessBootstrapPolicy.harness`,
  `setup_first_party_plugin(target: HarnessKind, ...)`,
  `setup_detected_plugin(target: HarnessKind, ...)`.
  This is exactly the closed-enum dispatch the parent feature's `HarnessKind`
  elimination was meant to remove.
- `crates/harnesses/src/lifecycle.rs` — the `NativeLifecycleRequest.harness:
  HarnessKind` field (line 81) that this story's own `Units` list explicitly
  says to drop ("drop `harness: HarnessKind` from `NativeLifecycleRequest`").
  Plus `lifecycle_scope.rs` tests and the field's construction sites.
- `crates/harnesses/src/lib.rs` — the seam itself plus three
  `detect_*_installation(harness: HarnessKind, ...)` functions and the
  `select_profile(harness: HarnessKind, ...)` compatibility wrapper.
- `crates/harnesses/tests/{bootstrap.rs,detection.rs,lifecycle_scope.rs}` —
  extensive direct construction and matching.

The active stories' scopes do not cover these:

- `registry-cli` Units bound it to `crates/cli/src/{command,entrypoint,
  application, application/*}.rs` only — it migrates the CLI's `HarnessKind`
  consumers, not the harnesses-crate producers.
- `registry-test-support` Units bound it to the test-support crate's
  `FakeNativeMode::CodexVersion / ClaudeVersion` removal; it does not migrate
  the harnesses crate's own integration tests' `HarnessKind` usage.

Consequence: even after `registry-cli` and `registry-test-support` reach done,
`HarnessKind` will remain alive in `crates/harnesses/{src,tests}/`, and the
literal acceptance criterion `git grep -n "HarnessKind" crates/` returns no
matches — shared by this story and the parent feature's Unit 2 — will continue
to fail at parent roll-up. The seam is not "merely staged"; its staging has a
real ownership hole that leaves the elimination target unowned.

This is a material design/acceptance blocker because the three-way approval
test the caller set (seam preserves behavior **AND** complete elimination is
explicitly and safely owned by the already-active CLI/test-support stories
**AND** parent acceptance will still enforce zero matches) requires all three;
the second conjunct fails.

### Important — the contract file was extended after its story terminalized

`crates/harnesses/src/registry.rs` is the `registry-contract` story's
deliverable, and that story is `done` with its trait shape approved. This
story modified that file to add two trait methods not present in the contract
design or its review:

- `HarnessAdapter::decode_version_with_limits(stdout, limits)` with a default
  that delegates to `decode_version(stdout)`. Functionally necessary: the
  original `decode_native_version(harness, stdout, json_limits)` carried the
  JSON boundary limits, and the contract's `decode_version(stdout)` dropped
  them, so the JSON-decode path needs the limits to preserve behavior.
- `NativeLifecycleVector::observation_scope(&self, scope: &Scope) ->
  Option<CapabilityScope>` — **no default impl**, so this is a breaking change
  to the trait. Functionally necessary: `lifecycle.rs::resource_observation`
  calls it to recover Claude's user/local scope disambiguation that the
  original `match request.scope { Global => "user", Project(_) => "local" }`
  provided.

The additions are defensible on the merits, but they extend the contract
surface after the fact and are not acknowledged in the contract story body or
its review. Either move them into a deliberate contract-amendment stride (and
update the contract story / its review to reflect the extended trait), or
fold them into this story's documented deviations from the parent design so
the contract file is not silently rewritten by a sibling story.

### Important — one hidden semantic rewrite in the lifecycle vector

`CodexLifecycle::arguments` reorders the rejection guards versus the original
`native_arguments`:

- Original: `validate_native_request` → `Codex + PluginUpdate =>
  UnsupportedAction` → per-harness match (`Codex if project =>
  UnsupportedProjectScope`).
- New: `validate_native_request` → `Scope::Project(_) =>
  UnsupportedProjectScope` → `PluginUpdate => UnsupportedAction`.

For `Codex + PluginUpdate + Project`, the original returned
`UnsupportedAction` ("the native harness has no verified lifecycle command");
the new returns `UnsupportedProjectScope` ("the native harness has no
verified project-scoped lifecycle command"). The existing tests do not cover
this combination, so the regression is unverified rather than caught. Both
outcomes are rejections, but the diagnostic and error code differ, and the
story's literal guarantee is "Relocate, do not rewrite … behavior is preserved
byte-for-byte." Restore the original precedence (PluginUpdate guard before
the project-scope guard) and add a regression test pinning the
Codex+PluginUpdate+Project outcome, or document the precedence change
explicitly as a deviation with justification.

### Verified

- Adapter trait / port method bodies preserve the Codex/Claude capability
  matrix exactly. `adapter_helpers::compiled_capabilities(false, false)` for
  Codex and `compiled_capabilities(true, true)` for Claude reproduce the
  previous `!codex` matrix for `plugin.update` and the entire project-scope
  row (verified against the prior `compiled_capabilities` in lib.rs).
- `adapter_helpers::select_profile` is the prior logic parameterized by
  `(verified_version, profile_id, capabilities)`; codex `0.144.1` /
  `codex-0-144-1` and claude `2.1.201` / `claude-2-1-201` are pinned by the
  table test in `adapters/mod.rs`, plus unknown-version `Unverified`
  fallback for both scopes.
- `adapter_helpers::decode_native_version` is the prior decoder factored by a
  `text_version` closure (`strip_prefix("codex-cli ")` /
  `strip_suffix(" (Claude Code)")`); control-char rejection, `
`/`
` strip,
  `is_single_version_token`, and the `{...}` strict-JSON path with caller
  limits are all preserved.
- `ClaudeLifecycle::arguments` is `claude_arguments` moved verbatim,
  including the `MarketplaceUpdate` skips-`--scope` special case and the
  user/local mapping.
- `CodexLifecycle::arguments` is `codex_arguments` moved verbatim *modulo the
  precedence change above*.
- `CodexInstructionBridge` / `ClaudeInstructionBridge` / `CodexSkillProjection`
  / `ClaudeSkillProjection` reproduce the prior per-harness bridge and skill
  destinations (`codex_home/AGENTS.md`, `claude_home/CLAUDE.md`, project
  `CLAUDE.md`, `.agents/skills`, `claude_home/skills`, `.claude/skills`).
- `HarnessKind::adapter()` resolves through `TargetRegistry::canonical()`, so
  the migration is dispatch-equivalent for the ports it covers.
- `TargetRegistry::canonical().ids()` yields exactly `codex`, `claude`; gemini
  returns `None`; first-party filter yields both.
- Verification reproduced: `cargo test -p skilltap-harnesses` → 55 passed
  (6 suites); `cargo clippy -p skilltap-harnesses --all-targets -- -D warnings`
  → clean; `cargo fmt -p skilltap-harnesses -- --check` → clean. Workspace
  `cargo check` has 16 errors confined to `crates/cli/` consuming the removed
  `HarnessPolicyMap.{codex,claude}` fields — that is the expected
  registry-cli integration seam.

### Resolution required

Pick one — either resolves the blocker:

1. Extend `registry-cli` (and/or `registry-test-support`) acceptance criteria
   to explicitly own `HarnessKind` elimination across `crates/harnesses/{src,tests}/`
   (including `bootstrap.rs`, the `NativeLifecycleRequest.harness` field, the
   seam in `lib.rs`, and the harnesses integration tests), and re-add this
   story's literal `git grep -n "HarnessKind" crates/` and CLI literal
   acceptance checks to whichever story completes the elimination. Then this
   story may re-enter review with the seam intact and the ownership chain
   explicit.
2. Do the harnesses-crate `HarnessKind` elimination in this story after all
   (its `Units` already lists dropping the `NativeLifecycleRequest.harness`
   field and removing `HarnessKind` from `lib.rs`), and re-scope the seam to
   CLI-only — i.e., leave only the `crates/cli/src/` consumers for
   `registry-cli`.

Either way, also: acknowledge the contract trait extension
(`decode_version_with_limits`, `observation_scope`) in the contract story or
this story's deviations, restore (or document) the
Codex+PluginUpdate+Project error precedence, and add the missing regression
test.

**Nits (parked, below the material bar)**: none worth filing while the
blocker is open — they would be re-evaluated on the next review pass.

## Review resolution (2026-07-12)

All receiver-confirmed findings from review commit `6c58e476` are resolved:

- **Ownership blocker — resolved.** Commit `3119913c` updates the durable
  `epic-expanded-harness-support-registry-cli` Units, implementation notes, and
  acceptance criteria to own final `HarnessKind` removal across CLI consumers,
  harnesses bootstrap/lib/lifecycle producers, harnesses integration tests, and
  `NativeLifecycleRequest.harness`. The temporary seam remains here solely to
  keep this intermediate story independently compilable.
- **Contract extensions — acknowledged and justified.** The implementation
  notes now explicitly record `decode_version_with_limits` as necessary to
  preserve caller-bounded strict JSON version decoding and
  `NativeLifecycleVector::observation_scope` as necessary to keep Claude's
  native `user`/`local` scope evidence adapter-owned and scope-safe.
- **Lifecycle precedence — resolved.** `CodexLifecycle::arguments` again rejects
  `PluginUpdate` before project scope, exactly matching the original
  `native_arguments` diagnostic. A direct regression test asserts that
  PluginUpdate+Project returns `UnsupportedAction`.

Receiver verification confirms every other migrated behavior remains
byte-equivalent: version parsing and limits, exact known/unknown capability
matrices, canonical observation roots, Codex/Claude direct argv, Claude scope
evidence, instruction bridges, and skill destinations are unchanged.

Review-fix verification: 56 harness tests passed across six suites; clippy with
`-D warnings`, formatting, and `git diff --check` all passed.
