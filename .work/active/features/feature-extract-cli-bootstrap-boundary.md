---
id: feature-extract-cli-bootstrap-boundary
kind: feature
stage: drafting
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

Awaiting the normal feature-design pass. The implementation should be staged
as small mechanical moves (composition/projection, binary publication and
rollback, then daemon policy and tests) so every intermediate commit remains
buildable and review can compare outcomes directly.

