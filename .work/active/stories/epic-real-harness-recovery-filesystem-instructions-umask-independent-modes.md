---
id: epic-real-harness-recovery-filesystem-instructions-umask-independent-modes
kind: story
stage: implementing
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: null
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
