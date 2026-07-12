---
id: story-skilltap-plugin-distribution-bootstrap-harness
kind: story
stage: review
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-contract]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Harness detection and first-party plugin setup

Implement the read-first per-target setup adapter for the canonical skilltap
plugin. Reuse the existing executable detection, verified capability profiles,
native lifecycle argument builders, and conservative JSON observation. This
story must preserve the Codex contract gap rather than inventing a plugin cache
write path.

Scope:

- `crates/harnesses/src/bootstrap.rs` and module exports.
- `crates/cli/src/bootstrap.rs` adapter wiring needed by the application.
- `crates/harnesses/tests/bootstrap.rs` fake-binary tests.

Acceptance criteria:

- `all` probes Claude and Codex independently; absent/unusable targets produce
  bounded results without hiding a successful binary result for another target.
- Claude uses a verified user-scoped native marketplace/plugin lifecycle for
  the canonical skilltap source; existing presence is observed first and a
  healthy repeat is a no-op.
- Codex may register the canonical marketplace only when its verified profile
  grants that operation. The adapter never claims a plugin install when the
  host exposes only an interactive flow and returns an actionable unsupported
  result instead.
- Unknown or malformed native list/version output blocks mutation, raw payloads
  and secrets stay inside the adapter, and no harness cache is written.
- Fake binaries assert exact direct argument vectors, scope isolation, profile
  narrowing, present/missing/unknown observation, and unsupported Codex setup.

Do not add arbitrary third-party selectors, project scope, harness enablement,
or an undocumented post-install hook.

## Implementation notes
- Execution capability: highest available local capability; this crosses native harness contracts and trust boundaries.
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/harnesses/src/bootstrap.rs`, `crates/harnesses/src/lib.rs`.
- Tests added: unknown-version observe-only and unreachable-target bounded-result tests; setup performs presence observation before any native mutation and returns actionable Codex/Claude next actions.
- Discrepancies from design: setup policy accepts the configured executable and canonical source as application-owned inputs, keeping target probing and native command composition in the adapter.
- Adjacent issues parked: none.

## Review findings (2026-07-12)

- **Blocker — canonical source and marketplace identity are dropped from plugin setup** (`crates/harnesses/src/bootstrap.rs:155-188`, `crates/harnesses/src/lifecycle.rs:450-458`): bootstrap constructs a `PluginInstall` request with `source: Some(https://github.com/nklisch/skilltap)` but the native argument builder ignores `source` for plugin installs and emits only `plugin install skilltap ...`. It never registers the canonical marketplace first and uses the unqualified name even though native plugin identity is marketplace-qualified. On a clean host the command can install from no selected source (or fail), and a present `skilltap@marketplace` will be classified missing and reinstalled on every run. Register/observe the canonical marketplace and use the exact qualified identity, or return an explicit unsupported/attention result when the native host cannot do so; add fake-binary tests for first setup and healthy repeat.
- **Blocker — Codex capability gap is not preserved at runtime** (`crates/harnesses/src/bootstrap.rs:144-160`, `crates/harnesses/src/lib.rs:566-624`): any exact `codex 3.0.0` profile grants `plugin.install`, so bootstrap proceeds to `codex plugin add skilltap` without a runtime capability probe or Codex-specific guard. A host that only exposes the documented interactive `/plugins` flow receives a mutation attempt and is classified as a generic command failure rather than the required actionable `Unsupported` result. Narrow the profile with observed help/capability evidence and ensure unsupported Codex setup never invokes a plugin-install vector.
- **Important — detected executable identity is not carried into mutation** (`crates/harnesses/src/bootstrap.rs:138-188`): after read-first detection, setup ignores the `ExecutableIdentity` in `HarnessInstallation` and resolves `policy.configured` again. A PATH replacement between the version probe and lifecycle call can run a different binary under the old profile. Bind lifecycle execution to the observed executable identity (or re-probe and revalidate the exact identity immediately before mutation), and test replacement between phases.
- **Important — malformed version diagnostics are reported as not-installed** (`crates/harnesses/src/bootstrap.rs:120-130`): every detection/probe error, including malformed or unknown version JSON, is collapsed to `SetupReason::NotInstalled`. Mutation is conservatively avoided, but the result is not truthful or actionable for an installed-but-unusable harness. Preserve an invalid/unknown-version reason distinct from absence.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: canonical marketplace/qualified plugin setup; runtime Codex unsupported-capability handling (this item)
**Important**: detected executable identity binding; truthful malformed-version diagnostics (this item)
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a native-contract/correctness pass. Workspace tests passed, but the read-first/native lifecycle and target-isolation lenses found that the canonical source is not actually used by the install vector and the Codex interactive contract gap is not represented. Item remains at `stage: implementing` pending fixes and fake-binary coverage.

## Review (2026-07-12, hardened follow-up)

**Verdict**: Request changes

**Blockers**: none beyond the missing acceptance evidence
**Important**: capability-bound marketplace mutation and native contract
regression coverage are incomplete (this item)
**Nits**: none

**Notes**: Standard fresh-context substrate review of commits `c880496` and
`85b56ea`. The implementation now preserves Codex as an actionable
`Unsupported` result, uses the canonical `.../tree/main/plugin` source and
qualified `skilltap@skilltap` identity for Claude, observes before mutation,
and binds mutations to the detected executable identity with last-moment
revalidation. It nevertheless gates only `plugin.install` before invoking
the marketplace add operation; a narrowed profile that withdraws
`marketplace.register` would still receive a native marketplace mutation.
The story also has no `crates/harnesses/tests/bootstrap.rs` fake-binary suite:
the two local unit tests do not prove exact marketplace/plugin vectors,
scope/target isolation, present/missing/unknown list behavior, capability
narrowing, or identity replacement handling. Add the operation-specific
capability check and the required isolated tests. Item remains at
`stage: implementing`.

## Review (2026-07-12, coverage follow-up)

**Verdict**: Request changes

**Blockers**: marketplace mutation is still authorized by `plugin.install`
alone; `marketplace.register` is not checked before the native add ->
`story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage`
**Important**: required fake-binary lifecycle/identity regression suite is
still missing -> `story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage`

**Nits**: none

**Notes**: Standard fresh-context review of `9e8ab3c`/`ea49bec`. Canonical
Claude source/qualified identity, read-first observation, Codex
`Unsupported`, and last-moment executable identity revalidation are present.
The adapter still checks only `plugin.install` and can invoke marketplace
registration when the selected profile does not grant `marketplace.register`.
The new integration test covers Codex unsupported only; no isolated fake
binary tests assert exact Claude vectors, scope/target isolation,
present/missing/unknown handling, capability narrowing, or replacement
blocking. Item remains at `stage: implementing` until the existing contract
coverage follow-up closes both gaps.

## Review (2026-07-12, fresh-context acceptance)

**Verdict**: Request changes

**Blockers**: none in the corrected guard (this review)
**Important**: operation-specific capability and fake-binary coverage remain
missing -> `story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage`

**Nits**: none

**Notes**: Standard fresh-context review after `00b9493`. The adapter now
requires both `marketplace.register` and `plugin.install` for Claude, keeps
Codex unsupported, binds native calls to the observed executable, and the
workspace tests are green. The harness suite still has only one Codex
unsupported assertion and one Claude happy-path log check; it does not prove
that a narrowed profile blocks marketplace registration, exact argument
vectors/scope and target isolation, present/missing/unknown observation,
malformed output, replacement blocking, or cache non-mutation. Keep this item
at `stage: implementing` until the existing contract-coverage follow-up adds
those isolated tests and capability seam.
