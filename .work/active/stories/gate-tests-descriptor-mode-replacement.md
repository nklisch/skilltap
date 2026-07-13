---
id: gate-tests-descriptor-mode-replacement
kind: story
stage: done
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Prove mode changes cannot follow a replaced destination

## Priority
High

## Spec reference
`epic-real-harness-recovery-filesystem-instructions-umask-independent-modes`:
mode setting uses the already-open descriptor and cannot follow a replaced
path.

## Required test
Replace the pathname after open and assert a symlink target's mode remains
unchanged while publication fails closed or cleans only the owned identity.

## Implementation notes
- Execution capability: inline; this is a focused adversarial hook and one filesystem test.
- Review weight: standard (project default).
- Files changed: `crates/core/src/runtime/filesystem/directory_tree.rs`, `crates/core/src/runtime/filesystem/directory_tree/tree_io.rs`, `crates/core/src/runtime/filesystem/directory_tree/tests.rs`.
- Tests added: pathname replacement after descriptor open leaves the symlink target at mode `0644`, applies executable mode only to the already-open owned inode, and returns an identity-change error.
- Discrepancies from design: none; the low-level publication writer is exercised directly so the replacement can be injected at the exact open-to-mode boundary.
- Adjacent issues parked: none.
- Verification: focused adversarial test, all 344 core tests/integration tests, and strict all-target core Clippy pass.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate Deep review at effective review weight `standard` (explicit
caller selection), performed in fresh context because the test exercises a
pathname-replacement race. The writer opens with `O_NOFOLLOW`, captures the
opened inode identity, performs `fchmod` on that descriptor, and verifies both
descriptor and pathname identity afterward. The adversarial hook replaces the
pathname with an external symlink after open; the external mode stays `0644`,
only the opened owned inode becomes `0700`, and verification fails on identity
change. Focused descriptor and directory-tree tests passed. No foundation-doc,
public-contract, or release drift found; product/UX lenses were inapplicable.
