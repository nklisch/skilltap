---
id: feature-extract-cli-bootstrap-boundary
kind: feature
stage: implementing
tags: [refactor, infra]
parent: null
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract the CLI bootstrap boundary

## Discovery finding

`crates/cli/src/entrypoint.rs` has grown a second application boundary for
binary bootstrap. The command dispatcher is followed by the complete release
resolution, binary publication, lock coordination, post-publish identity
probe, rollback/cleanup implementation, result projection, and daemon binary
policy (approximately lines 285-1920). This is more than half of the file and
mixes command routing with filesystem publication and update-policy details.

The binary publication primitives in `crates/core/src/runtime/artifact.rs`
remain the domain/runtime port. The CLI code composes those ports and renders
outcomes; it is therefore a CLI boundary rather than a reason to add concrete
harness or terminal concerns to core.

## Classification

Pure refactor: move the existing bootstrap composition and its focused tests
behind a private CLI module without changing resolver/fetcher/installer
selection, lock ordering, artifact verification, rollback behavior, daemon
policy, output fields, or exit/result classes. No new update or installer
capability belongs in this item.

## Value

The extraction makes command dispatch and the existing bootstrap contract
scannable, gives binary publication one obvious CLI owner, and prevents future
daemon or installer changes from growing `entrypoint.rs` further. It also
keeps test-only composition seams next to the code that owns them while
leaving the core artifact boundary unchanged.

## Target shape

Create a private `crates/cli/src/bootstrap_commands.rs` (or an equivalently
named private module) that owns:

- `execute_system_bootstrap` and bootstrap outcome composition;
- binary execution modes, resolver/fetcher/installer composition, and the
  configuration lock boundary;
- publication identity probes, rollback/cleanup helpers, and their focused
  test fixtures; and
- the daemon binary update-policy projection and destination helper.

Keep `entrypoint::run_from` as the stable dispatcher. It should call narrow
`pub(super)` bootstrap wrappers, and `daemon_commands` should call the same
daemon binary-policy wrapper rather than reaching into bootstrap internals.
The module may use existing `crate` outcome/rendering types and core runtime
ports, but must not introduce a second artifact installer or native harness
implementation.

## Guardrails

- Preserve the canonical HTTPS resolver and bounded transport; fixture ports
  remain `#[cfg(test)]` and ambient environment variables cannot replace the
  production resolver or artifact source.
- Preserve configuration-lock acquisition/release ordering, including the
  pending result on contention and attention result on release failure.
- Preserve atomic publication, destination identity checks, no-clobber
  rollback, and cleanup behavior on first install and update races.
- Preserve binary major-version policy, daemon `off`/`check`/`apply-safe`
  behavior, and the exact destination selected by an installed daemon
  service.
- Preserve plain/JSON resource ordering, warning/error codes, next actions,
  summaries, and exit/result classification byte-for-byte where applicable.
- Keep `crates/core/src/runtime/artifact.rs` and `install.sh` behaviorally
  unchanged; a change to those contracts is a separate feature, not a
  refactor of this boundary.
- Run the existing bootstrap, daemon binary-policy, rollback-race, and
  installer parity tests unchanged before and after each extraction step.

## Rejected candidates

Deduplicating the shell installer with its static verifier, changing native
installation capability checks, consolidating core artifact rollback helpers,
or altering bootstrap diagnostics would change testing/ownership or public
behavior and are not part of this pure refactor.

## Design status

Designed for the normal refactor-design pass. The implementation is staged as
small mechanical moves (composition/projection, binary publication and
rollback, then daemon policy and tests) so every intermediate commit remains
buildable and review can compare outcomes directly.

## Design decisions

- Keep `crates/cli/src/bootstrap_commands.rs` private to the CLI crate. The
  module owns the bootstrap command boundary and binary publication policy; it
  does not become a public library API and does not move release/artifact ports
  into `skilltap-core`.
- Keep `entrypoint::run_from` as the only dispatch owner. Its `Bootstrap`
  arm calls a narrow `bootstrap_commands::execute_system_bootstrap` wrapper;
  daemon execution calls a separate `pub(super)` binary-policy wrapper. No
  caller reaches through the module for resolver, installer, rollback, or
  filesystem helpers.
- Move the focused `bootstrap_tests` module with the code it exercises. Test
  ports remain injected only in `#[cfg(test)]` helpers; production wrappers
  continue constructing the canonical resolver, bounded fetcher, installer,
  and configuration lock themselves.
- Preserve helper bodies and visibility as mechanically as possible. A
  `pub(super)` result/policy wrapper is acceptable where `entrypoint` or
  `application` needs the boundary; no new generic artifact or daemon utility
  module is warranted.
- Keep the binary policy and daemon destination lookup in the same module so
  foreground bootstrap and daemon updates cannot drift. `daemon_commands`
  continues to own service lifecycle and service-manager behavior; it may only
  call the narrow policy wrapper.
- Treat every output detail as contract: resource order, summary keys,
  warning/error codes, next actions, plain/JSON rendering, result classes,
  lock contention, rollback outcomes, and byte-level diagnostics are unchanged.

## Refactor Overview

Bootstrap currently accounts for roughly 1,600 lines of `entrypoint.rs`,
including native harness setup, release resolution, binary publication,
identity probing, race-safe rollback, daemon update policy, and all focused
fixtures. This leaves command dispatch coupled to the riskiest filesystem
publication code and requires daemon code to reach through entrypoint-private
helpers. A private `bootstrap_commands` boundary makes the ownership explicit
without introducing a new runtime abstraction or changing any public command
behavior.

The extraction is intentionally incremental. The first step establishes the
module and moves command composition/result projection while the low-level
helpers remain callable through a temporary parent-private bridge. The second
step moves the resolver/fetcher/installer, lock, identity, temporary workspace,
and rollback implementation plus its fixture ports. The final step moves
daemon binary policy and destination discovery, updates the two callers to
the narrow wrappers, and removes all bootstrap-only code/imports from
`entrypoint.rs`.

## Refactor Steps

### Step 1: Extract bootstrap command composition and outcome projection

**Priority**: High
**Risk**: Medium
**Source Lens**: code smell / missing abstraction (command dispatch mixed with first-party setup)
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/bootstrap_commands.rs`, `crates/cli/src/entrypoint/tests.rs`
**Story**: `story-feature-extract-cli-bootstrap-boundary-composition`

**Current State**:

`entrypoint.rs:285-457` owns `execute_system_bootstrap`,
`compose_bootstrap_outcome`, `BinaryBootstrapResult`, and the target/config
selection loop that detects configured Codex and Claude binaries and invokes
`setup_first_party_plugin`. The dispatch arm calls the private function
directly, while all binary publication helpers remain interleaved immediately
after it.

**Target State**:

Declare a private `bootstrap_commands` module and move the command-level
composition, harness setup loop, outcome projection, result type, and the two
composition tests into it. Expose only:

```rust
pub(super) fn execute_system_bootstrap(args: &BootstrapArgs) -> Outcome
```

The module may temporarily call the still-parent
`execute_binary_bootstrap` through a narrow `pub(super)` bridge until Step 2;
the bridge is removed when the binary implementation moves. `run_from` keeps
its existing argument and rendering behavior and only changes the dispatch
path to `bootstrap_commands::execute_system_bootstrap`.

**Implementation Notes**:

- Preserve missing-config defaults, target narrowing, absolute-versus-PATH
  binary resolution, bounded process/JSON limits, canonical source locator,
  and the rule that harness setup is skipped when binary bootstrap is an
  attention result.
- Preserve the binary resource first, harness resource ordering, target
  filtering, warning/next-action construction, and summary fields exactly.
- Keep test-only fixture construction and `compose_bootstrap_outcome` tests
  inside the new module; do not broaden visibility solely for tests.
- Do not move daemon service lifecycle code or alter the separate
  `execute_system_daemon_run` reconciliation composition in this step.

**Acceptance Criteria**:

- [ ] `entrypoint.rs` dispatches bootstrap through the private module and no
      duplicate command-composition function remains.
- [ ] First-party setup with missing config, one target, both targets, and a
      blocked binary produces byte/structure-equivalent plain and JSON output.
- [ ] Composition tests pass with unchanged assertions, resource ordering,
      target narrowing, warning codes, next actions, and exit classes.
- [ ] The intermediate tree builds and `cargo fmt --all -- --check` passes.

**Risk**: A module-private bridge can accidentally change import resolution or
target filtering while moving the command loop.
**Rollback**: Revert the module/dispatch move and restore the composition block
and tests to `entrypoint.rs`; no native files or state are touched.

---

### Step 2: Move binary publication, locking, identity, and rollback support

**Priority**: High
**Risk**: High
**Source Lens**: code smell / missing abstraction (large publication boundary with race-sensitive helpers)
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/bootstrap_commands.rs`, `crates/cli/src/entrypoint/tests.rs`
**Story**: `story-feature-extract-cli-bootstrap-boundary-publication`
**Depends On**: `story-feature-extract-cli-bootstrap-boundary-composition`

**Current State**:

`entrypoint.rs:439-1229` owns `BinaryBootstrapResult`, execution modes and
targets, canonical resolver/fetcher/installer construction, configuration-lock
coordination, release decision and checksum/permission/identity validation,
temporary workspaces, race-safe atomic rollback/cleanup, executable identity
and version probes, and binary attention/pending projections. The focused
`bootstrap_tests` module exercises these helpers through test-only resolver,
fetcher, installer, and lock ports.

**Target State**:

Move the full binary publication boundary and its test module into
`bootstrap_commands.rs`. The module exposes only the narrow wrappers needed by
the command and daemon policy:

```rust
pub(super) fn execute_binary_bootstrap(...)
pub(super) fn execute_binary_bootstrap_for_daemon(...)
```

All resolver/fetcher/installer generic seams, rollback helpers, identity
probes, temporary path helpers, result structs, and execution-mode types stay
private to the module. `entrypoint.rs` retains no duplicate artifact
installer, filesystem race logic, or publication outcome constructor.

**Implementation Notes**:

- Copy function bodies mechanically and preserve the canonical HTTPS resolver,
  bounded process limits, `SKILLTAP_INSTALL` handling, artifact-key checks,
  major-version decision, checksum verification, executable permissions,
  private temporary workspace, and atomic installer invocation.
- Preserve lock directory creation, contention as pending, release failure as
  attention, and all result fields and warning/next-action text.
- Preserve post-publication identity checks and no-clobber rollback semantics:
  inode identity is captured after installation; replacement races remain
  untouched; residual cleanup is retained when identity cannot be proven.
- Re-home all binary matrix, daemon-check, lock-contention, custom-target,
  rollback-race, cleanup-race, wrong-identity, and non-executable fixtures
  unchanged. Test ports stay `#[cfg(test)]` and production code cannot select
  them through environment variables.
- Remove only imports made dead by the move; avoid unrelated formatting or
  helper deduplication in the same change.

**Acceptance Criteria**:

- [ ] `entrypoint.rs` contains no binary publication, lock, rollback, identity,
      temporary-workspace, or binary attention/pending implementation.
- [ ] Install/no-op/update/major-block/major-opt-in, check-mode, lock
      contention, wrong release identity, permission, publication identity,
      rollback replacement, and cleanup tests pass unchanged.
- [ ] Foreground bootstrap keeps identical resource fields, policy labels,
      result classes, warning codes, and next actions in plain and JSON modes.
- [ ] `cargo test -p skilltap-cli --offline`, formatting, and `git diff --check`
      pass before the daemon policy move.

**Risk**: Visibility or import changes can subtly alter race handling or make
rollback bless an unrelated executable.
**Rollback**: Revert only the publication move and restore the helper/test block
to `entrypoint.rs`; no state or installed binary changes are retained by the
source-only rollback.

---

### Step 3: Move daemon binary policy and complete the boundary

**Priority**: High
**Risk**: Medium
**Source Lens**: missing abstraction / leaky private boundary (daemon reaches into bootstrap internals)
**Files**: `crates/cli/src/entrypoint.rs`, `crates/cli/src/bootstrap_commands.rs`, `crates/cli/src/daemon_commands.rs`, `crates/cli/src/application/lifecycle.rs`, `crates/cli/tests/compiled_binary.rs`
**Story**: `story-feature-extract-cli-bootstrap-boundary-daemon-policy`
**Depends On**: `story-feature-extract-cli-bootstrap-boundary-publication`

**Current State**:

`entrypoint.rs:1804-1962` owns the `daemon run` binary update policy,
`daemon_binary_destination`, and `binary_policy_attention`. The daemon run
entrypoint calls that private code before reconciliation, while the policy
reuses the foreground publication helpers directly. This leaves daemon
behavior coupled to the dispatch module and makes a future bootstrap change
require edits in two unrelated sections of `entrypoint.rs`.

**Target State**:

Move `execute_system_daemon_binary_policy`, its attention projection, and
`daemon_binary_destination` into `bootstrap_commands.rs`. Expose one
`pub(super)` policy function returning the existing `Outcome`. Keep
`execute_system_daemon_run` in `entrypoint.rs` as a thin composition wrapper
that calls the module policy, then invokes `execute_system_reconciliation`.
The module remains the sole caller of binary publication internals; the
`daemon_commands` service lifecycle module is unchanged.

**Implementation Notes**:

- Preserve bootstrap update mode `off`/`check`/`apply-safe`, persisted
  `allow_major`, daemon service destination parsing, lock path selection,
  policy labels, `binary_changed`/`binary_pending` summaries, warning codes,
  and attention result classes exactly.
- Preserve launchd/systemd service-root and executable extraction through the
  existing `crate::daemon` helpers. Do not move service ownership validation or
  manager operations into the bootstrap module.
- Keep compiled `daemon run` help and lifecycle tests unchanged; add no new
  daemon-specific output or retry behavior. The existing application lifecycle
  merge continues to consume the same `Outcome` summary/resource shape.
- Remove all bootstrap-only imports and dead helper definitions from
  `entrypoint.rs` after the module compiles. The final dispatch surface should
  be scannable: command arms, thin daemon run composition, and unrelated CLI
  commands only.

**Acceptance Criteria**:

- [ ] `entrypoint.rs` retains only a thin `daemon run` wrapper; daemon binary
      policy, destination lookup, and binary helpers live in
      `bootstrap_commands.rs`.
- [ ] `daemon run` off/check/apply-safe, disabled/missing/malformed service,
      lock contention, update, and attention outputs remain structurally and
      textually equivalent, including application lifecycle merge behavior.
- [ ] Compiled leaf, daemon, bootstrap, and application lifecycle tests pass
      without assertion changes; repeated daemon cycles remain idempotent.
- [ ] Full workspace formatting, offline tests, strict clippy, and diff checks
      pass, with no changes to `skilltap-core` artifact contracts or `install.sh`.

**Risk**: Moving the policy wrapper can change when binary updates run relative
to reconciliation or alter the service destination error mapping.
**Rollback**: Restore the policy block and thin wrapper in `entrypoint.rs`; no
daemon service files or binary state are changed by the source-only revert.

## Implementation Order

1. `story-feature-extract-cli-bootstrap-boundary-composition`
2. `story-feature-extract-cli-bootstrap-boundary-publication` (depends on Step 1)
3. `story-feature-extract-cli-bootstrap-boundary-daemon-policy` (depends on Step 2)

## Children

- `story-feature-extract-cli-bootstrap-boundary-composition`
- `story-feature-extract-cli-bootstrap-boundary-publication`
- `story-feature-extract-cli-bootstrap-boundary-daemon-policy`
