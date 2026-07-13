---
id: epic-real-harness-recovery-filesystem-instructions-umask-independent-modes
kind: story
stage: done
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Publish exact managed modes independently of umask

## Finding

Descriptor-relative artifact publication currently supplies `0700` or `0600`
to `openat`, but those creation bits are reduced by the process umask. A caller
running with a restrictive umask can therefore publish an intended executable
without its owner execute bit, or a regular managed file without the exact
private owner permissions promised by the artifact contract.

## Required fix

- After securely creating and identity-validating the descriptor, set its mode
  through the open descriptor to exactly `0700` for executable artifacts and
  `0600` otherwise; do not reopen by path.
- Keep no-follow and identity revalidation before and after content
  publication, and propagate any mode-normalization failure through the
  existing partial-publication cleanup path.
- Add a serialized test that temporarily installs a restrictive process umask,
  restores it reliably even on failure, publishes both file kinds, and proves
  exact `0700`/`0600` modes plus unchanged repeat/idempotence behavior.
- Run the full workspace suite, formatting check, and all-target/all-feature
  clippy after the focused filesystem tests.

## Acceptance

- Exact managed file modes do not depend on the invoking process umask.
- Mode setting uses the already-open descriptor and cannot follow a replaced
  path.
- Publication failure remains fail-closed and cleans only the proven owned
  destination.

## Implementation notes

- Execution capability: direct inline implementation; the change is narrowly
  owned by the descriptor-relative publication boundary but security-sensitive.
- Review weight: standard (project default), with full workspace verification
  after the focused restrictive-umask regression.
- Files changed: `crates/core/src/runtime/filesystem/directory_tree/tree_io.rs`
  and its colocated tests.
- Tests added: isolated single-test child execution under umask `0777`, exact
  `0600`/`0700` assertions, typed reload, and unchanged repeat behavior; the
  existing publication test now also asserts the repeat is an immutable
  `AlreadyExists` outcome.
- Discrepancies from design: the regression invokes `write_tree` against a
  pre-opened fixture directory so owner execute bits can be stripped from file
  creation without also making newly created parent directories inaccessible.
  The production path remains the complete `publish_tree` cleanup pipeline.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core 'runtime::filesystem::directory_tree::tests::' -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context substrate review at the project-default `standard` weight using the deep lane for descriptor-relative permission handling. Commit `a0bb263` applies exact `0600`/`0700` modes through the already-open descriptor, retains identity checks and cleanup propagation, and proves behavior in an isolated child with umask `0777`. Focused core tests passed.
