---
id: story-centralize-stable-hash
kind: story
stage: done
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Centralize stable FNV-1a hashing

## Value

Remove the repeated private FNV-1a loops used for operation, instruction,
backup, checkout, lifecycle, and native resource identifiers. One helper keeps
the hashing algorithm consistent without changing any prefixes or observable
IDs.

## Scope

Extract `stable_hash(&str) -> u64` in `crates/cli/src/application.rs` and
replace the equivalent loops around lines 4482-4512, 4698-4702, 4857-4892,
5224-5227, and 6616-6619. Preserve exact input labels and outputs.

## Acceptance

- All existing identifier tests remain green.
- No public signatures or serialized IDs change.
- The repeated hash implementation exists only once.

## Implementation Notes

- Added one private `stable_hash(&str) -> u64` FNV-1a helper and replaced all
  operation, instruction, backup, checkout, lifecycle, skill, and native
  resource identifier loops with it while preserving labels and formatting.
- Verification: `cargo fmt --all` and `cargo test -p skilltap --offline`
  passed (40 unit tests and 41 compiled-binary tests).

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard same-harness fresh-context review. The helper preserves the
FNV-1a seed, byte order, wrapping arithmetic, labels, prefixes, and formatting;
the workspace fmt, tests, clippy, and diff checks are green.
