---
id: epic-expanded-harness-support-registry-test-support
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
  - epic-expanded-harness-support-registry-adapters
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reusable Adapter Acceptance Contract

## Scope

Implement Unit 5 of the registry feature design. Make test-support derive
isolated roots and fake executable profiles from the registry instead of from
per-harness `FakeNativeMode` variants, and codify the shared acceptance matrix
that every registered adapter must pass.

## Units

- `crates/test-support/src/harness_profile.rs` (new):
  - `VersionResponse { TextPrefix, TextSuffix, Json }`.
  - `LifecycleDialect { Codex, Claude, None }`.
  - `FakeHarnessProfile { id, version_response, lifecycle_dialect }` with
    `codex()` / `claude()` constructors and a `build(root, behavior)` that
    composes a harness-specific version/lifecycle script with a generic
    `FakeNativeMode` process-behavior mode.
  - `acceptance_matrix(profile, machine) -> AcceptanceReport` running the
    HARNESS-CONTRACTS "Adding Another Harness" criteria: detection, both
    scopes, complete skills, MCP observation, reload, drift, removal, and
    immediate-repeat idempotency.
- `crates/test-support/src/native_process.rs` (modified):
  - Remove `FakeNativeMode::CodexVersion` and `FakeNativeMode::ClaudeVersion`.
  - Move the lifecycle script block currently gated by
    `matches!(mode, CodexVersion | ClaudeVersion | VersionKnown)` behind
    `LifecycleDialect` so any future Codex-like or Claude-like adapter reuses
    it without a new branch.
  - Keep the generic process-behavior modes (`Hang`, `Flood`, `ProbeNarrow`,
    `ProbeDrift`, `MalformedJson`, `DuplicateJson`, `ExtraJsonDocument`,
    `RetainPipes`, `Exit`, `VersionKnown`, `VersionUnknown`) since they are
    orthogonal to harness identity.
- `crates/test-support/src/lib.rs` (modified): re-export the new types.
- Existing tests that constructed `FakeNativeMode::CodexVersion` /
  `ClaudeVersion` migrate to `FakeHarnessProfile::codex().build(...)` /
  `claude().build(...)`.

## Implementation notes

- `FakeHarnessProfile::codex().build(root, FakeNativeMode::VersionKnown)` must
  produce an executable whose `--version` output is byte-identical to today's
  `FakeNativeMode::CodexVersion` (and likewise `claude()` vs `ClaudeVersion`).
- `acceptance_matrix` is the reusable contract adapter features will invoke
  with their own profile. This story populates and exercises it for Codex and
  Claude only; it does not add profiles for other targets.

## Acceptance criteria

- [x] `FakeHarnessProfile::codex().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's removed Codex version mode.
- [x] `FakeHarnessProfile::claude().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's removed Claude version mode.
- [x] Every existing test that constructed a Codex or Claude fake is migrated
      to the profile constructor; affected test-support and harnesses suites pass.
- [x] `acceptance_matrix(&FakeHarnessProfile::codex(), machine)` passes the full
      fixture-level detection/scope/skill/mcp/reload/drift/removal/idempotency
      suite, and likewise for `claude()`.
- [x] `git grep` over `crates/` returns no removed identity-specific fake-mode
      call sites. Repository-wide matches are historical references in the
      parent/sibling work-item designs, which this story was explicitly barred
      from editing.

## Out of scope

- Profiles or acceptance runs for any target other than Codex and Claude.
- Changes to the production registry, adapter, config, or CLI modules beyond
  what migrating the existing fakes requires.

## Implementation record

- Execution capability: highest, selected by the active autopilot caller because
  the fixture API is shared by every future adapter. Review weight remains the
  caller's `standard`.
- Dispatch: direct implementation only, as required by the caller. No delegated
  agent or peer mechanism was used.
- Added dependency-neutral `VersionResponse`, `LifecycleDialect`, and
  `FakeHarnessProfile`. `test-support` stores a validated static id rather than
  importing production `HarnessId`: core and harnesses already dev-depend on
  test-support, so reversing that dependency would create a Cargo package cycle.
  Codex and Claude constructors are the test-side projection of the canonical
  registry identities and compose identity with orthogonal process behavior.
- Removed the Codex/Claude identity branches from `FakeNativeMode`. Profiled
  `VersionKnown` responses preserve exact version bytes; the existing generic
  JSON `VersionKnown` behavior remains unchanged for non-profiled boundary tests.
  Lifecycle emulation is gated by `LifecycleDialect`, while retaining the
  existing command/list state machine so current lifecycle tests remain stable.
- `FakeHarnessProfile::build` materializes under a caller-owned isolated root.
  Executable publication prefers the prior hard-link behavior and falls back to
  an exact copy only when the build artifact and isolated root cross filesystems.
- Added a proportional fixture-level `acceptance_matrix` rather than duplicating
  the compiled CLI suite or adding a forbidden production dependency. It creates
  documented Codex/Claude global and project skill/MCP surfaces, observes all
  files in complete skills, performs fresh MCP reload reads, detects supporting-
  file drift, removes both resource types, and proves immediate repeat produces
  no writes. Existing harness detection tests and compiled CLI tests remain the
  production-bound adapter/command evidence.
- Migrated every existing identity-specific fake call site in harness detection
  and compiled CLI tests to `FakeHarnessProfile`. The staged `HarnessKind` seam
  and all production CLI/harness/core code remain untouched for the next story.

## Verification

- `cargo test -p skilltap-test-support -p skilltap-harnesses` — passed, 75 tests
  across 8 suites.
- `cargo clippy -p skilltap-test-support -p skilltap-harnesses --all-targets -- -D warnings`
  — passed.
- `cargo fmt --all -- --check` — passed.
- `git grep -n -E 'FakeNativeMode::CodexVersion|FakeNativeMode::ClaudeVersion' -- crates`
  — no matches.
- `cargo test -p skilltap --test compiled_binary` — expected Unit 3 → Unit 4
  integration blocker: 16 CLI production compile errors still access removed
  `HarnessPolicyMap.codex` / `.claude` fields. This story did not alter those
  out-of-scope production sites; the ready CLI integration story owns that seam.

## Review (2026-07-12)

**Verdict**: Bounce
**Review weight**: standard, risk-escalated to focused Deep fresh-context
because this story defines the reusable acceptance evidence every future
adapter feature will claim against.
**Reviewer context**: cross-model — Z.AI GLM 5.2 fresh-context review of an
OpenAI-host run (different model class).

### Blocker — the new profile build path introduces an intermittent
ETXTBSY execve race that makes the story's own verification non-reproducible
and will regress the migrated compiled-binary suite

`FakeHarnessProfile::build(root, behavior)` (`harness_profile.rs:99`) is the
new caller-owned-root entry point. Unlike the legacy
`FakeNativeProcess::new` path — which built the TempRoot under `OUT_DIR` and
hard-linked the stable fixture on the **same filesystem** — the profile path
builds under the caller's isolated machine root (typically `/tmp`, a separate
filesystem from `OUT_DIR` on `/storage`). `publish_executable`
(`native_process.rs:512-520`) therefore falls back from `fs::hard_link` to
`fs::copy` on every cross-device build, then the caller immediately
`execve`s the freshly copied script (`native_process.rs` shell wrapper +
sourced `behavior`).

Under default `cargo test` parallelism that copy-then-execve sequence flakes
with `Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`.
Measured flake rate on this machine: **3 of 12** runs of
`cargo test -p skilltap-test-support --lib` failed; the failing tests are
the two this story adds (`profile_versions_and_lifecycle_scripts_preserve_native_bytes`
and `codex_and_claude_pass_the_reusable_acceptance_matrix`) plus the
negative-root test. The panic is at `harness_profile.rs:355:69`, i.e.
`native.command().arg("--version").output().unwrap()` — the `execve`, not
the build. `git grep` confirms the removed `FakeNativeMode::CodexVersion` /
`ClaudeVersion` call sites are gone, and the panic is on executing the new
profile-built binary, not on constructing it.

Root cause verified, not assumed:

- A minimal parallel repro that calls the real `FakeHarnessProfile::codex()` /
  `claude().build(TempRoot, VersionKnown)` then immediately `execve`s emits
  `ExecutableFileBusy` for ~3% of iterations across 8 threads x 48 builds, with
  **every failing destination path unique** (distinct sequence numbers under
  `NEXT_TEMP_ROOT`). The race is therefore not a path collision; it is the
  copy-then-execve window itself.
- The legacy `FakeNativeProcess::new` path never hit this because it built and
  executed entirely on the `OUT_DIR` filesystem via `fs::hard_link`; no
  cross-device copy, no transient writer on the executed inode.
- Wrapping the `execve` in a bounded retry on `io::ErrorKind::ExecutableFileBusy`
  (200 tries x 200us) eliminates every failure in the repro, confirming the
  race is transient and the fix is mechanical.

Why this is material, not a parked nit:

1. The story's literal acceptance criterion — "affected test-support and
   harnesses suites pass" — is not satisfiable: the new tests fail on roughly
   a quarter of default-parallelism runs, and the story's verification block
   ("passed, 75 tests across 8 suites") is not reproducible.
2. The migrated `crates/cli/tests/compiled_binary.rs` call sites now construct
   fakes through `fake_harness(machine, profile)` -> `profile.build(machine.working_directory(), ...)`
   and invoke `fixture.executable()` directly, i.e. they execute the same
   cross-device-copied binary this story introduced. The CLI seam currently
   masks this (16 `HarnessPolicyMap.{codex,claude}` compile errors block the
   binary build), but the moment `epic-expanded-harness-support-registry-cli`
   removes the seam, the previously-stable compiled-binary suite inherits
   exactly this flake. That is a regression in production-bound evidence
   caused by this story's fixture change, not by the CLI story.
3. The whole purpose of Unit 5 is a reusable acceptance contract that future
   adapter features will invoke and rely on for green CI. A contract whose
   happy path intermittently fails with `Text file busy` will produce
   false-negative CI failures unconnected to the adapter under test — the
   precise "creates false confidence" (and false alarm) failure mode the
   caller asked this review to rule out.

Required fix (small, scoped to `crates/test-support`):

- Make the cross-device publication path robust to the execve race. The
  minimal change is to retry on `io::ErrorKind::ExecutableFileBusy` in the
  test-support execution surface (a thin helper around
  `FakeNativeProcess::command()` execution, or a documented retry at each
  `.output()`/`.status()` site the profile tests drive). Alternatives that
  also resolve it: `fsync` the destination before returning from
  `publish_executable`, or copy-to-temp-then-rename with an explicit
  read-only `chmod`. The implementor should pick one and prove it with a
  parallel-stress test that runs the profile build+exec loop under
  contention without `ExecutableFileBusy`.
- Re-run `cargo test -p skilltap-test-support --lib` to green at least 10
  consecutive times under default parallelism before re-entering review, and
  record that evidence in the verification block.

The production-bound detection/observation/CLI contracts are unaffected and
the compiled-binary regression is latent only because of the unrelated CLI
seam; the fix is confined to test-support.

### Verified sound (no change required)

- **Version byte preservation** (`harness_profile.rs:18-29`, test
  `profile_versions_and_lifecycle_scripts_preserve_native_bytes`):
  `VersionResponse::render()` yields `codex-cli 0.144.1\n` and
  `2.1.201 (Claude Code)\n` byte-for-byte; the generic non-profile
  `VersionKnown` still emits `{"version":"0.144.1"}`. `shell_quote_value`
  embeds the literal newline correctly so `printf '%s'` reproduces the prior
  bytes.
- **Lifecycle dialect gate** (`native_process.rs:371-373`): the block runs for
  any non-`None` dialect and for the legacy generic `VersionKnown` (no
  profile), so existing lifecycle tests stay stable and the script body is
  unchanged.
- **Generic modes orthogonal** (`native_process.rs:24-42`): `Hang`, `Flood`,
  `ProbeNarrow`, `ProbeDrift`, `MalformedJson`, `DuplicateJson`,
  `ExtraJsonDocument`, `RetainPipes`, `Exit`, `VersionKnown`,
  `VersionUnknown` are unchanged and compose with a profile via
  `FakeHarnessProfile::builder(behavior)`.
- **Migration completeness**: `git grep -n 'CodexVersion|ClaudeVersion' crates/`
  returns no matches; all identity-specific call sites in
  `crates/{cli/tests/compiled_binary.rs,harnesses/tests/detection.rs}` now use
  `FakeHarnessProfile`. The detection cross-harness case
  (`HarnessKind::Codex` decoded against a Claude profile and vice-versa) is
  preserved, and the previously-mixed `ExtraJsonDocument` case is split into
  its own focused assertion — a readability improvement, not a regression.
- **Isolated-root safety** (`harness_profile.rs:103-108`, test
  `profile_build_requires_a_caller_owned_isolated_root`): a non-existent root
  returns `NotFound` before any fixture is written.
- **`acceptance_matrix` scope is honest, not tautological**: the routine does
  real filesystem work (materialize, snapshot-tree drift, fresh MCP reload
  read, removal verification, immediate-repeat idempotency via
  `write_if_changed`) and explicitly documents itself as fixture-level,
  deferring production-bound detection/observation/mutation to the existing
  harnesses and compiled-binary suites. It does not claim to exercise
  skilltap's production drift/update/removal paths, so it does not create
  false confidence on the production contract. "Update identity" from
  HARNESS-CONTRACTS `Adding Another Harness` is honestly out of the
  matrix's listed scope and remains covered by the compiled-binary
  `native_plugin_and_marketplace_lifecycle` tests.
- **Path realism** (`harness_profile.rs:120-153`): global/project skill and
  MCP roots for codex (`.agents/skills/...`, `.codex/config.toml`) and claude
  (`.claude/skills/...`, `.claude/settings.local.json` project /
  `claude_home/settings.json` global) match the harness observation paths.
- **CLI compile blocker correctly unowned here**: the 16
  `HarnessPolicyMap.{codex,claude}` errors are the expected Unit 3 -> Unit 4
  seam and are durably owned by `epic-expanded-harness-support-registry-cli`
  (including final `HarnessKind` and `NativeLifecycleRequest.harness`
  removal); this story did not touch production CLI/harness/core code beyond
  migrating the fakes.
- **Tooling**: `cargo clippy -p skilltap-test-support -p skilltap-harnesses
  --all-targets -- -D warnings` clean; `cargo fmt -p skilltap-test-support -p
  skilltap-harnesses -- --check` clean.

### Notes (non-blocking, address if convenient during the fix stride)

- `publish_executable`'s hard-link-then-copy fallback is the only new
  production-of-fixtures code path; consider documenting inline why the copy
  fallback exists (cross-device `OUT_DIR` vs caller-owned isolated root) and
  why the execve race it creates is handled by the retry, so a future reader
  does not "simplify" the retry away.
- The lifecycle gate at `native_process.rs:371-373` now also emits the
  lifecycle block when a profile is composed with a non-`VersionKnown` mode
  (e.g. `Hang`, `Exit`) because `lifecycle_dialect != None`. No current test
  exercises that combination and the block is argv-gated so it is benign, but
  it is a behavior expansion versus the pre-story
  `CodexVersion|ClaudeVersion|VersionKnown`-only gate. A one-line comment
  noting the intent would prevent surprise.

### Resolution required

Fix the cross-device copy/execve race in `crates/test-support` so the new
profile tests are deterministically green under default parallelism (demonstrated
by >=10 consecutive clean `cargo test -p skilltap-test-support --lib` runs and a
parallel-stress test), then re-enter review. No other change is required; the
matrix, migration, version bytes, lifecycle gate, path realism, and CLI-seam
ownership are all sound.
