# Target-local resource state

Mutate lifecycle evidence on exact harness bindings while preserving every
unselected sibling binding.

## Rationale

One logical plugin or skill can have different native IDs, revisions,
provenance, ownership, and journal state in Codex and Claude. Resource-wide
replacement would erase that distinction.

## Examples

- Target map and sibling-preserving primitives:
  `crates/core/src/storage/state.rs:636`
- Journal and available-revision updates: `crates/core/src/storage/state.rs:845`
- Observation refresh merging: `crates/core/src/storage/state.rs:969`
- Foreground update recording: `crates/core/src/foreground_update.rs:321`
- Publication of exact target bindings: `crates/core/src/publication.rs:326`

These paths rebuild only the selected target state and carry all other target
bindings forward unchanged.

## When to Use

- State, update, publication, reconciliation, or removal narrowed by target.
- Dual-native resources whose harness identities or revisions differ.

## When NOT to Use

- Truly resource-wide policy such as the logical desired resource key.
- Values proven identical and invariant across every target.

## Common Violations

- Storing target provenance or revision at resource level.
- Reconstructing a resource from selected targets only.
- Clearing sibling journal evidence during a target-scoped operation.
