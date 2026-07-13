# Identity-aware rollback with residual evidence

Capture the owned identity or expected representation before mutation, restore
only that object, then re-observe and report exact residual state when recovery
cannot be proven.

## Rationale

Rollback must not overwrite a replacement created by another process or claim
success while partially restored surfaces remain. Failure output is part of
the recovery contract.

## Examples

- Binary publication preserves replacement identities:
  `crates/cli/src/bootstrap_commands.rs:517`
- Managed project rollback restores in reverse order and reports residuals:
  `crates/cli/src/application/execution.rs:438`
- Directory publication cleanup reports identity and durability uncertainty:
  `crates/core/src/runtime/filesystem/directory_tree.rs:800`

Each path captures pre-mutation evidence, limits cleanup to owned identities,
and distinguishes complete restoration from residual or durability uncertainty.

## When to Use

- Multi-step publication, replacement, or removal.
- Any rollback where another process may replace a pathname.

## When NOT to Use

- Pure computation or read-only observation.
- Unmanaged resources that skilltap has no authority to restore.

## Common Violations

- Freshly statting and blessing a replacement as the original object.
- Unconditional deletion during cleanup.
- Discarding restore errors or omitting residual paths.
- Claiming rollback success without post-observation.
