---
id: epic-rust-control-plane-storage-managed-artifacts
kind: story
stage: implementing
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

## Review findings

Fresh-context deep review requested corrections before approval:

- use the Apple `__error()` errno location rather than Linux
  `__errno_location()` under macOS cfg;
- serialize directory namespace creation for cooperating skilltap writers and
  verify created/opened/path identities before writing, after writing, and
  before treating an inode as owned;
- report exact destination presence and parent-directory durability, including
  failures before destination open/identity and unlink-success/sync-failure;
- make backup collisions retry rather than escape on the first mismatched stale
  PID/sequence path;
- re-check descriptor file type after open so raced FIFO/device/socket entries
  are rejected before reading or removal; and
- preserve the caller's publish/backup action when an occupied path cannot be
  loaded or compared.

Portable POSIX APIs cannot exclude a malicious same-UID process that ignores
advisory locks and continuously replaces directory names. The supported safety
contract is therefore the same as the configuration lock: exclusive parent
advisory locking for cooperating skilltap CLI/daemon writers, descriptor-bound
no-follow operations, and identity verification at every namespace boundary.
Tests must deterministically exercise each former seam and every residual state.

Re-review confirmed all semantic findings closed but found one remaining macOS
compile blocker: Apple exposes signed `stat.st_dev` while `DirectoryIdentity`
uses normalized `u64`. Raw stat device/inode values must use explicit checked
conversion at construction and comparison; Linux-only source scanning is not a
substitute for the platform type contract.

That conversion is corrected, but final re-review reproduced a distinct
parallel-suite failure: successful managed publication intermittently returns a
generic runtime failure while isolated and serial runs pass. This must be
root-caused rather than retried or hidden. Safe action/error-kind
instrumentation is allowed for diagnosis; the fix needs a deterministic
regression, at least 30 consecutive parallel full-core passes, and the complete
locked ladder.

## Review corrections

- Portability: directory enumeration now clears errno through Apple `__error()`
  under `target_vendor = "apple"` and `__errno_location()` elsewhere; a
  source-level guard test requires both cfg branches.
- Cooperating writers: publish and remove hold an exclusive advisory lock on
  the durable `managed/` root for the complete namespace operation. A
  deterministic contention test proves a second writer fails before creation.
- Identity proof: every created directory is `mkdirat` → `fstatat` → no-follow
  `openat` → descriptor-type check → path identity check. Destination and file
  identities are verified before writes, after file sync, and before success.
- Entry safety: recursive load/remove re-check descriptor file type immediately
  after nonblocking open. Deterministic post-stat swaps to FIFOs are rejected
  without reading, unlinking, or hanging; the replacement FIFO remains.
- Residuals: partial directory errors and managed residuals now carry exact
  destination presence (`present`, `removed`, or `unknown`), optional observed
  device/inode, and parent-directory sync state. Injected cleanup tests cover a
  present owned inode after removal refusal and removed destination with
  uncertain durability after unlink succeeds but parent sync fails.
- Occupied paths: publish retains `Publish` across occupied load/compare errors.
  Backup retries occupied stale/mismatched candidates, including load failures,
  and only reports `Backup` conflict after 32 exclusive candidates are exhausted.
- Structure: recursive Unix tree I/O moved to private `tree_io.rs`; production
  directory-tree modules are 368, 176, and 234 lines.
- Tests added by correction: eight deterministic identities (Apple cfg, writer
  lock, two FIFO races, two residual states, publish action, backup retry).
  All 128 pre-correction workspace identities remain and 136 now pass.
- Corrected verification passed: locked format, all-target check,
  warnings-denied Clippy, 136 workspace tests, warnings-denied rustdoc, exact
  identity comparison, and diff hygiene.

## Final portability correction

- Raw `libc::stat` device and inode values now flow through one checked generic
  `u64` normalizer. Both identity construction and path comparison consume the
  normalized `DirectoryIdentity`; no raw signed/unsigned comparison remains.
- The production normalizer is instantiated in tests with Linux-style unsigned
  values and Apple-style signed `dev_t` values. Positive identities normalize
  identically, while negative signed device or inode values fail explicitly.
- No Apple Rust target is installed in the environment (only
  `x86_64-unknown-linux-gnu`), so an actual Apple target check was not possible
  without mutating the global toolchain. The typed signed-shape test exercises
  the production helper rather than relying on source-string inspection.
- All 136 pre-correction workspace identities remain; one typed portability
  identity was added and 137 tests pass. The full locked ladder and three
  additional consecutive full workspace test runs pass.
