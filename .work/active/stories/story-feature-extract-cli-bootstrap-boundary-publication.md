---
id: story-feature-extract-cli-bootstrap-boundary-publication
kind: story
stage: done
tags: [refactor, infra]
parent: feature-extract-cli-bootstrap-boundary
depends_on: [story-feature-extract-cli-bootstrap-boundary-composition]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract binary publication and rollback support

## Brief

Move the complete binary bootstrap publication boundary from
`entrypoint.rs` into `bootstrap_commands.rs`, including resolver/fetcher/
installer composition, lock coordination, release decision, identity probes,
temporary workspace, and race-safe rollback/cleanup. Keep all test-only ports
and fixtures beside the extracted implementation.

## Current / target

`entrypoint.rs:439-1229` currently owns `BinaryBootstrapResult`, execution
modes and targets, production resolver/fetcher/installer construction,
configuration-lock handling, artifact verification and publication, identity
checks, private temporary paths, rollback exchanges/no-replace cleanup,
version probing, and attention/pending projections. The nested
`bootstrap_tests` module exercises every race-sensitive path.

After this story, those definitions and tests have one private owner in
`bootstrap_commands.rs`. Only narrow command/daemon wrappers cross the module
boundary. Generic resolver, fetcher, installer, lock, and rollback helpers
remain private; test injection remains `#[cfg(test)]`.

## Guardrails

- Preserve canonical HTTPS release resolution, bounded transport and process
  limits, `SKILLTAP_INSTALL`, artifact-key and checksum checks, executable
  permissions, and major-version policy.
- Preserve lock directory creation, contention-as-pending, release-failure
  attention, result fields, warning codes, next actions, and output ordering.
- Preserve post-publication identity capture, exchange/no-replace rollback,
  replacement preservation, residual cleanup, and fail-closed unsupported
  platform behavior exactly.
- No core artifact port, shell installer, or native harness capability changes.

## Acceptance criteria

- [ ] `entrypoint.rs` contains no binary publication, lock, identity,
      temporary-workspace, rollback, or binary attention/pending definitions.
- [ ] Install/no-op/update/major block and opt-in, check mode, lock
      contention, wrong identity, permission, rollback-race, and cleanup-race
      tests pass without assertion changes.
- [ ] Foreground bootstrap output remains structurally and textually
      equivalent in plain and JSON modes.
- [ ] CLI tests, formatting, and `git diff --check` pass before Step 3.

## Risk / rollback

A visibility or import mistake could change race handling or permit clobbering
an unrelated executable. Revert this source-only extraction and restore the
helper/test block to `entrypoint.rs`; no installed binary or persisted state is
part of the rollback.

## Implementation notes

- Execution capability: highest available local implementation; mechanical
  extraction preserved every publication helper and fixture.
- Review weight: standard (autopilot default).
- Files changed: `crates/cli/src/bootstrap_commands.rs`,
  `crates/cli/src/entrypoint.rs`.
- Tests added: none; binary matrix, lock, identity, rollback, cleanup, and
  daemon-check fixtures moved unchanged with their implementation.
- Discrepancies from design: this publication boundary moved in the same
  source-only commit as composition and daemon policy to avoid an intermediate
  duplicate implementation; no runtime or output contract changed.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap --offline` (59 unit/compiled/package tests passed)
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings`
- `git diff --check`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate review, standard weight. Publication, lock, identity,
rollback, cleanup, and test-only seams are private to `bootstrap_commands`;
the full CLI test suite and strict clippy pass with unchanged assertions and
no core artifact or installer changes.
