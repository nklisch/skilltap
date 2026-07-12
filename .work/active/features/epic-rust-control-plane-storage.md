---
id: epic-rust-control-plane-storage
kind: feature
stage: done
tags: [infra]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-runtime-primitives]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Machine State Storage

## Brief

Implement versioned repositories for `config.toml`, `inventory.toml`,
`state.json`, and skilltap-owned artifacts beneath the resolved machine-wide
configuration directory. Reads validate full documents and reject unknown
skilltap-owned fields; writes validate complete replacements and atomically
publish them so readers observe either the old or new document.

The repositories model missing first-use state, managed artifact ownership, and
recoverable backup locations without storing authentication material. This
feature does not observe harness-native files, calculate reconciliation plans,
or perform resource lifecycle operations.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: repository and schema consumer of the shared domain and
  runtime filesystem contracts; follows runtime primitives so atomic
  publication has one implementation

## Foundation references

- `docs/SPEC.md` — Configuration Directory, `config.toml`, `inventory.toml`,
  `state.json`, `managed/`, Validation
- `docs/ARCH.md` — Storage, Concurrency, Error Model
- `docs/VISION.md` — Core Idea, Audience, Observable Ownership

## Design

### Storage boundary

Storage lives in `skilltap-core::storage` and depends only on domain and runtime
ports. It exposes explicit typed schemas and repository traits for each owned
document; a private document engine may share missing/present reads,
decode/encode, complete validation, and atomic publication, but no public
untyped or generic persistence API exists. Repositories never observe harness
files, plan changes, acquire the process lock, write terminal output, or perform
resource lifecycle operations.

Every owned schema has its own `schema = 1`, rejects unknown fields, validates
the complete document on construction/deserialization and again before write,
and serializes deterministically. A missing document is `Missing`, not an empty
or default document. Reads never create the configuration directory; the first
successful replacement creates it. Present malformed or unsupported documents
are typed errors and are never repaired implicitly.

### Initial schemas

`ConfigDocument` is strict TOML operating policy:

- `schema`
- Codex and Claude `HarnessPolicy { enabled, binary }`
- `InstructionPolicy { claude_mode: symlink | import }`
- `UpdatePolicy { mode: off | check | apply-safe, interval }`

Defaults match the foundation (`codex`, `claude`, symlink, apply-safe, `6h`),
but are returned only by an explicit constructor. `UpdateInterval` is a
canonical positive integer plus `s|m|h|d`; no scheduler behavior enters storage.
Binary values use the existing bounded native identifier and may be a PATH name
or absolute configured string. Authentication/environment material has no
field.

`InventoryDocument` is strict TOML desired state:

- `schema`
- deterministic canonical project-root set
- deterministic `DesiredResource` list covering marketplaces, plugins,
  standalone whole-directory skills, and instruction locations

The constructor sorts by `ResourceId`, rejects duplicates, validates the full
desired dependency graph, and requires every project-scoped resource path to be
declared in the project set. Existing domain values remain the single source of
truth for targets, source/requested ref, update intent/pin, component choices,
accepted consequences, and dependencies. If direct serde produces unreadable
or unsupported TOML, explicit strict wire structs convert to/from the domain;
validation is not weakened.

`StateDocument` is strict JSON observation/provenance state:

- `schema`
- harness states (native version and observation time)
- resource states keyed by stable `ResourceId`
- last update check, successful observation, and successful application times

State resource records carry native IDs, provenance/source, ownership, optional
managed-artifact record, fingerprints, installed/available resolved revisions,
observation time, and the last per-resource operation results. They do not carry
desired targets, component policy, or update policy. A validated nonnegative
Unix timestamp with nanosecond precision converts to/from the runtime clock.
Operation IDs are unique per apply record. Managed artifact paths are unique,
require skilltap ownership and direct/materialized provenance, and are forbidden
for unmanaged ownership.

### Managed artifacts

`managed/` stores complete owner-bound directory trees. A managed record has a
resource owner, role (`materialized_plugin`, `direct_skill`, or `backup`), a
relative path beneath `managed/`, and optional fingerprint. An `ArtifactTree`
is a deterministic map of validated relative file paths to bytes; the whole
skill directory is the artifact, including its top-level `SKILL.md` when the
caller publishes a standalone skill. Storage does not inspect skill semantics.

Trees publish immutably to unique owner/fingerprint paths while the process lock
is held by the application layer. Files are fully written and synced before a
state document may reference the tree. Existing destinations never overwrite;
failure removes only owned partial paths or returns exact residual context.
Updates publish a new tree and atomically switch state to the new record; old
owned trees can then be removed explicitly. This avoids claiming atomic
replacement of a non-empty directory. Every derived path is proven beneath
the non-symlink managed root, and load/remove require exact owner/path matches.
Backups use generated unique owned paths and never overwrite.

### Error model

`StorageError` distinguishes document kind and action (read, decode, validate,
encode, write), unsupported schema versions, ownership/path conflicts, runtime
filesystem failures, and managed publication or removal residuals. Error display includes
only safe document/path context, never document contents, native stdout, or
secrets.

### Pre-mortem

- **TOML cannot represent a tagged domain shape readably.** Spike a complete
  inventory first; use explicit wire conversions rather than loosening domain
  validation or storing JSON inside TOML.
- **Defaults hide corrupt state.** Missing remains explicit and present
  documents require every top-level section; defaults are opt-in only.
- **State becomes a second desired source.** State schemas forbid target,
  component-choice, and update-policy fields; inventory alone is authoritative.
- **Managed removal escapes through a link.** Derive all paths from the managed
  root, inspect each owned boundary without following, verify owner/path, and
  adversarially test live/dangling ancestor links and traversal.
- **A directory update becomes partially visible.** Publish immutable new trees
  before state references them; never replace a referenced non-empty tree in
  place.

## Implementation units

1. `epic-rust-control-plane-storage-schemas` — strict versioned config,
   inventory, state, timestamp, and artifact records/wires — depends on `[]`.
2. `epic-rust-control-plane-storage-document-repositories` — typed missing/
   present config, inventory, and state repositories through runtime atomic file
   publication — depends on `[epic-rust-control-plane-storage-schemas]`.
3. `epic-rust-control-plane-storage-managed-artifacts` — immutable owner-bound
   complete-tree and backup repository beneath `managed/` — depends on
   `[epic-rust-control-plane-storage-schemas]`.
4. `epic-rust-control-plane-storage-integration` — isolated machine-root
   contract tests across all repositories — depends on
   `[epic-rust-control-plane-storage-document-repositories,
   epic-rust-control-plane-storage-managed-artifacts]`.

## Acceptance criteria

- Missing first-use state is explicit and reads create nothing.
- All three document schemas independently version, reject unknown/malformed
  input, validate complete domain invariants, and serialize deterministically.
- Inventory expresses every foundation desired-resource primitive without
  duplicating domain state; state records observation/provenance only.
- Document replacement delegates to the one runtime atomic writer and repeated
  identical replacement is byte-stable and semantically idempotent.
- Managed trees contain complete directories, cannot escape/follow the managed
  root, never overwrite, and require matching owner/path for removal.
- Corruption in one repository does not rewrite or mask another; no storage
  file contains authentication material.
- Full locked format/check/Clippy/test/rustdoc ladder passes.

## Implementation summary

All eight children are complete. Storage now provides strict schema-1 config,
inventory, state, timestamp/apply, and artifact records; typed missing/present
TOML/JSON document repositories with descriptor-bound reads and atomic writes;
immutable owner/fingerprint-bound complete-tree publication, exact load/remove,
unique backups, and structured partial recovery through descriptor-relative
Unix operations; plus real-adapter machine-root integration tests. The
implementation performs no harness observation, planning, locking policy,
resource lifecycle, skill semantic validation, or discovery. The four
fresh-context review findings are corrected: schema versions evolve
independently, managed records share one canonical path contract, failed lock
acquisition releases provisional locks explicitly, and partial removals report
exact recovery state. The locked workspace passes 150 tests plus doctests and
warnings-clean rustdoc.

## Feature review findings

Fresh-context cross-contract review requested four corrections:

1. Config, inventory, and state use one shared schema version despite the
   architecture requiring independent evolution; repository probing must bind
   to the selected document's version.
2. `StateDocument` accepts arbitrary managed record paths that the managed
   repository refuses, including the committed golden; record construction and
   serde must share the repository's canonical owner/role/fingerprint/path
   validator.
3. Failed configuration-lock acquisition still relies on implicit unlock of
   provisional directory/file descriptors and intermittently poisons an
   immediate parallel acquisition.
4. Managed removal can partially delete contents or unlink before a failed
   parent sync yet returns only generic runtime failure; recovery needs exact
   presence, expected/observed identity, content-progress, and durability.

5. `epic-rust-control-plane-storage-independent-versions` — split and bind
   config/inventory/state schema versions — depends on
   `[epic-rust-control-plane-storage-document-repositories]`.
6. `epic-rust-control-plane-storage-managed-record-contract` — unify canonical
   managed record construction/serde/repository validation — depends on
   `[epic-rust-control-plane-storage-managed-artifacts]`.
7. `epic-rust-control-plane-storage-lock-release` — explicitly release
   provisional configuration locks on every failed acquisition path — depends
   on `[epic-rust-control-plane-storage-document-repositories]`.
8. `epic-rust-control-plane-storage-removal-residuals` — report exact partial
   managed-tree removal recovery state — depends on
   `[epic-rust-control-plane-storage-managed-artifacts]`.

## Review

Approved after a fresh-context re-review of the complete corrected boundary.
Independent schema versions, canonical managed-record validation, explicit
provisional-lock release, and exact partial-removal residuals were verified
together. The full locked format/check/Clippy/test/rustdoc ladder passes with
150 workspace tests, and 30 consecutive 16-thread core runs exercised 3,840
failed-lock/immediate-reacquire cycles without recurrence.
