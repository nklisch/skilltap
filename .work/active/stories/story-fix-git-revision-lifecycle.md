---
id: story-fix-git-revision-lifecycle
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Resolve Git refs and advance same-tree revisions

## Symptom

Installing from a non-default Git ref fails because the shallow clone does not
contain the requested ref. A later commit with identical skill contents is
reported as unchanged and the installed revision remains stale.

## Root cause

The source resolver verifies a requested ref without fetching it, while the
install no-op path compares only materialized content and does not refresh
provenance when the Git commit changes.

## Fix approach

Fetch an explicitly requested ref into the bounded checkout, resolve the
fetched commit, and refresh state/projections when the source revision changes
even when the copied tree is identical. Include old/new revision metadata in
update output.

## Regression test

`crates/cli/tests/compiled_binary.rs` covers a named feature ref and an empty
commit that advances the source SHA without changing the skill tree.

## Implementation notes

- Requested Git refs are fetched into the bounded checkout and resolved from
  `FETCH_HEAD` using direct argv execution.
- Same-tree update operations refresh source provenance without rewriting the
  materialized tree and report old/new revisions, compatibility, and targets.
- Added the non-default-ref and same-tree SHA regression.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast-lane substrate review. The non-default-ref and same-tree SHA
regressions plus green full workspace verification were confirmed; no lens walk
was needed for this story.
