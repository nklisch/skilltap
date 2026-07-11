---
id: epic-rust-control-plane-storage-managed-artifacts
kind: story
stage: review
tags: [infra]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-schemas]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Managed Artifact Trees

Implement immutable, owner-bound complete-directory publication and recoverable
backup storage beneath the resolved `managed/` root.

## Acceptance criteria

- `ArtifactTree` deterministically owns every file in the directory and rejects
  empty, duplicate, absolute, traversal, or non-normal paths; nested bytes
  round-trip exactly.
- Publication derives a unique owner/fingerprint path under `managed/`, writes
  and syncs every file before success, never overwrites, and returns exact owned
  residual context on cleanup failure.
- Live/dangling links at the root or any owned ancestor are rejected without
  following; paths cannot escape through spelling or races supported by the
  runtime boundary.
- Load and remove require matching owner/path; removal never touches an
  unowned/replaced inode. Backup paths are generated uniquely and never replace.
- A representative standalone skill preserves its top-level `SKILL.md` and all
  sibling content as one tree; storage adds no skill discovery/validation.
- Repeated publish reports existing immutable content rather than rewriting;
  full locked verification passes.

## Implementation notes

- Files changed: new `storage/managed_artifact.rs` and test sidecar; new narrow
  runtime `filesystem/directory_tree.rs` port/adapter with private Unix support;
  runtime/storage exports and errors; workspace SHA-256 dependency and lockfile.
- Storage API: added deterministic `ArtifactTree`, owner/record/inode-bound
  handles, `Published` versus `Existing` outcomes, exact loaded trees, explicit
  removal, unique backups, and safe contextual managed-artifact errors.
- Paths: immutable publications use one bounded filesystem-safe leaf derived
  from role, SHA-256(owner), and fingerprint. Backups use an exclusive generated
  leaf derived from SHA-256(owner), process ID, and sequence. The exact owner
  remains in the record, handle, errors, and residuals; maximum-length and
  distinct-owner cases are covered.
- Runtime boundary: added descriptor-relative exclusive `mkdirat`/`openat`
  publication, descriptor-bound recursive reads, and identity-checked
  `unlinkat` removal. Files and every created directory edge are synced before
  success. Live/dangling links and non-regular/empty foreign entries are never
  traversed. The final tree is never replaced.
- Failure ownership: the durable `managed/` root is repository structure; each
  artifact is a single child, so publication creates no owner-parent residue.
  Failed tree writes remove only the descriptor-owned final inode; cleanup
  failure returns exact owner/path/device/inode residual context.
- Skill boundary: the representative tree preserves top-level `SKILL.md`,
  scripts, references, binary bytes, and all siblings without parsing or
  validating skill semantics and without discovery behavior.
- Tests added: six adversarial tests covering validation/determinism,
  whole-skill bytes and immutable repetition/conflict, exclusive backups and
  bounded owner keys, owner/path/inode enforcement and path swaps, live and
  dangling links, and exact cleanup residuals.
- Test inventory: HEAD `8971383` had 122 live workspace test identities (the
  requested 99 count was stale after approved storage work). All 122 are
  preserved and exactly six scoped identities were added, for 128 total.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked` (128 passed), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`.
- Verification note: one existing lock path-swap test failed once in the first
  parallel full-suite run, then passed twice in isolation and in two subsequent
  full runs without a code change; no production issue was reproduced.
- Discrepancies from design: none. The required race safety could not be built
  from the existing path-based filesystem port, so the explicitly permitted
  narrow directory-tree runtime primitive was added.
- Adjacent issues parked: none.
