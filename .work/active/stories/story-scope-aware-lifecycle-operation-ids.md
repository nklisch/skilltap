---
id: story-scope-aware-lifecycle-operation-ids
kind: story
stage: review
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

# Include concrete scope in lifecycle operation identity

The native lifecycle operation identifier currently hashes the target and
resource id but not the concrete scope. A project-scoped and global resource
with the same id therefore produce duplicate operation ids when a command
uses `--all-scopes`; `plugin remove ... --all-scopes` fails closed with
`operation_plan_invalid` before mutation. Include the scope in the operation
identity and retain the all-scopes regression coverage in
`crates/cli/tests/compiled_binary.rs`.

## Implementation scope

Include the concrete scope in lifecycle operation identity generation while
preserving deterministic IDs for the same target/resource/scope tuple. Keep
the all-scopes regression coverage and verify global/project resources with the
same name produce distinct operations.

## Source

Promoted from `idea-scope-aware-lifecycle-operation-ids` after the release
e2e test exposed the production defect.

## Implementation notes

- Lifecycle operation hashing now includes the concrete scope label alongside
  the action, target, and resource id, so global and project operations with
  the same logical id remain distinct while repeated tuples stay deterministic.
- Added focused unit coverage for global/project identity separation and
  deterministic regeneration.
- Verification: `cargo test -p skilltap --lib application::tests:: --offline`
  passed (8 tests).
- Production commit: `9bc22a7` (`Fix scope-aware lifecycle operation IDs`).
