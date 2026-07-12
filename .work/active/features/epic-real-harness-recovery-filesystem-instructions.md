---
id: epic-real-harness-recovery-filesystem-instructions
kind: feature
stage: implementing
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: [epic-real-harness-recovery-runtime-boundary]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Preserve skill executability and correct instruction bridges

## Brief

Preserve safe executable semantics for complete skill directories and compute
instruction bridge targets relative to the actual canonical file for arbitrary
supported `HOME`/`CODEX_HOME` layouts. Health checks must resolve and compare
the effective link target, and an acknowledged repair that leaves no blocker
must complete successfully after creating its recoverable backup.

This feature owns blocker inventory entries 13-15. It does not change native
plugin lifecycle or aggregate update/status projection.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: consumes the shared runtime/root model; state/diagnostics
  consumes its final result semantics.

## Foundation references

- `docs/SPEC.md` — standalone skill model, instruction lifecycle, and mutation
  safety.
- `docs/ARCH.md` — standalone skills and instruction management.
- `docs/HARNESS-CONTRACTS.md` — canonical global instruction bridges.

## Design decisions

- **Executable fidelity boundary:** carry one normalized owner-executable intent
  bit with every regular artifact file from descriptor-relative observation
  through validation, fingerprinting, managed backup, publication, and drift
  observation. Any source execute bit sets the intent; destinations are `0700`
  for executable files and `0600` otherwise. Group/world access, write bits,
  set-id, sticky, ACL, and other special metadata are not propagated.
- **Instruction target identity:** derive every symlink target from the bridge
  parent and the actual canonical `AGENTS.md` path. Health compares the
  lexically resolved effective destination with that canonical absolute path
  and requires the destination to be a regular file; raw link text alone is
  never evidence of health.
- **Result semantics:** acknowledgment authorizes a repair but does not remain
  pending attention after the backup and replacement succeed. Final result is
  derived from operation results plus post-apply health; informational backup
  disclosure does not force exit 2.
- **Dispatch rationale:** direct-read design across the bounded artifact-tree
  and instruction-application surfaces. No exploratory fanout was needed after
  mapping the existing observer, `ArtifactTree`, descriptor-relative
  publication, instruction helpers, and compiled-binary tests.
- **Foundation timing:** code-first. The foundation contract already requires
  complete skill preservation, correct canonical bridges, safe repair, and
  idempotence; implementation only needs to remove code drift from it.

## Architectural choice

Use typed end-to-end intent and expectation values in core, with POSIX mode
observation/publication and symlink inspection remaining filesystem-adapter
concerns. This preserves Ports & Adapters: core decides which metadata is
meaningful and whether an observed bridge is healthy, while adapters only
capture or apply bounded machine facts.

Two alternatives were rejected. Copying the full source mode would retain more
metadata but would also reproduce unsafe group/world access, write permission,
and special bits. Inferring executability from path or shebang at publication
would avoid a contract change but is lossy for binaries and intentionally
non-executable scripts. For instructions, special-casing custom `CODEX_HOME`
with another fixed `../` depth would leave the same bug at a different layout;
the canonical relative-path computation is smaller and general.

The trickiest unit is executable intent because the current byte-only
`ExternalTreeEntry -> ArtifactTree -> DirectoryTreeFileSystem` pipeline also
defines fingerprints, backup equality, replacement rollback, and drift. The
intent must enter at descriptor-relative observation and remain in the same
authoritative artifact value everywhere; a side map or publication-only flag
would allow state and filesystem equality to disagree.

## Implementation units

### Unit 1: Normalized executable artifact contract

**Story:** `epic-real-harness-recovery-filesystem-instructions-executable-intent`

**Files:**

- `crates/core/src/domain/artifact.rs`
- `crates/core/src/domain/mod.rs`
- `crates/core/src/runtime/observation.rs`
- `crates/core/src/runtime/external_tree.rs`
- `crates/core/src/storage/managed_artifact.rs`
- `crates/core/src/storage/managed_artifact/tree_validation.rs`
- `crates/core/src/runtime/filesystem/directory_tree.rs`
- `crates/core/src/runtime/filesystem/directory_tree/tree_io.rs`
- `crates/core/src/skill.rs`
- affected core/CLI construction sites and tests that consume `ArtifactTree`

```rust
#[derive(Clone, Eq, PartialEq)]
pub struct ArtifactFile {
    contents: Vec<u8>,
    executable: bool,
}

impl ArtifactFile {
    pub fn new(contents: impl Into<Vec<u8>>, executable: bool) -> Self;
    pub fn contents(&self) -> &[u8];
    pub const fn is_executable(&self) -> bool;
}

impl ExternalTreeEntry {
    pub(crate) fn file(
        path: RelativeArtifactPath,
        bytes: Vec<u8>,
        executable: bool,
    ) -> Self;
    pub const fn file_executable(&self) -> Option<bool>;
}

pub struct ArtifactTree {
    files: BTreeMap<RelativeArtifactPath, ArtifactFile>,
}

impl ArtifactTree {
    pub fn new<P, F>(files: impl IntoIterator<Item = (P, F)>)
        -> Result<Self, ArtifactTreeError>
    where
        P: Into<String>,
        F: Into<ArtifactFile>;
    pub const fn files(&self)
        -> &BTreeMap<RelativeArtifactPath, ArtifactFile>;
}

pub trait DirectoryTreeFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError>;
    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(
        DirectoryIdentity,
        BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ), RuntimeError>;
}
```

**Implementation notes:**

- Keep `ArtifactFile` in the domain layer so storage and runtime adapters share
  one value without making runtime depend on storage. Its `Debug` output must
  omit contents and report only byte count plus executable intent.
- The external-tree adapter derives `executable` from
  `st_mode & 0o111 != 0` on the already-opened, identity-verified regular file.
  Re-check identity after reading as today; mode and bytes come from the same
  observed entry.
- `ValidatedSkillTree` retains this value for every file. Fingerprints encode
  path, normalized executable byte, length, and contents in an unambiguous
  order so a mode-only change is drift/update evidence.
- Descriptor-relative publication opens files with `0700` or `0600` according
  to intent. Descriptor-relative loading reconstructs the same normalized bit.
  Directories remain private `0700`; no source permission beyond execute intent
  crosses the boundary.
- Managed backups, replacement rollback, equality, and idempotence use the same
  artifact values. Do not add a parallel executable-path set.

**Acceptance:**

- [ ] A source file with any execute bit arrives as owner-executable at every
      global/project Codex and Claude skill destination.
- [ ] A non-executable source file remains non-executable even with a shebang or
      executable-looking path.
- [ ] Group/world, write, set-id, and sticky bits are not propagated.
- [ ] Mode-only source changes alter the skill fingerprint and are detected as
      update/drift; repeat publication is a no-op.
- [ ] Backup and rollback preserve normalized executable intent.
- [ ] Symlinks, devices, FIFOs, and source-entry races remain rejected without
      following or mutating them.

### Unit 2: Canonical instruction bridge specification and health

**Story:** `epic-real-harness-recovery-filesystem-instructions-relative-bridges`

**Files:**

- `crates/core/src/domain/artifact.rs` (relocated relative-target value)
- `crates/core/src/runtime/filesystem.rs`
- `crates/core/src/instructions.rs`
- `crates/cli/src/application.rs`
- `crates/cli/src/application/instructions.rs`
- `crates/core/src/runtime/filesystem/tests/ownership.rs`
- `crates/cli/tests/compiled_binary.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelativeSymlinkTarget(String);

pub fn relative_symlink_target(
    bridge: &AbsolutePath,
    canonical: &AbsolutePath,
) -> Result<RelativeSymlinkTarget, InstructionPathError>;

pub fn resolve_symlink_target(
    bridge: &AbsolutePath,
    observed: &Path,
) -> Result<AbsolutePath, InstructionPathError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionBridgeSpec {
    pub canonical: AbsolutePath,
    pub bridge: AbsolutePath,
    pub mode: InstructionBridgeMode,
    pub representation: InstructionBridgeRepresentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstructionBridgeRepresentation {
    Symlink(RelativeSymlinkTarget),
    Import(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedInstructionBridge {
    Missing,
    Symlink {
        effective_target: Option<AbsolutePath>,
        target_exists: bool,
    },
    RegularFile { fingerprint: Fingerprint },
    Other,
}

pub fn classify_bridge(
    spec: &InstructionBridgeSpec,
    observed: &ObservedInstructionBridge,
) -> InstructionHealth;
```

**Implementation notes:**

- Move the validated relative-target value from the filesystem adapter to the
  domain and re-export only if needed for compatibility inside the workspace.
  Relative computation compares normalized absolute path components from the
  bridge parent, emits the required leading parents plus canonical suffix, and
  fails before mutation if either path lacks a valid parent/component model.
- Resolve an observed relative link lexically from the bridge parent. Reject
  absolute targets and root escape; normalize `.`/`..` only for observation.
  Creation still uses the stricter normalized `RelativeSymlinkTarget`.
- Build one `InstructionBridgeSpec` per materialized bridge. Status, preview,
  setup, repair, and sync all consume it instead of independently choosing
  `../AGENTS.md`, `AGENTS.md`, or import bytes.
- A symlink is managed only when its effective absolute target equals
  `spec.canonical`, `target_exists` is true, and the canonical path is observed
  as a regular file. A dangling or equivalently spelled link to another file is
  broken/divergent, never managed.
- Preserve existing root-vs-nested project Claude selection. For each selected
  bridge, derive its expectation from its actual path; do not attach path-depth
  special cases to global/project labels.

**Acceptance:**

- [ ] Default homes still produce the minimal expected links and repeat as a
      no-op.
- [ ] A sibling custom `CODEX_HOME`, a deeper custom root, and a root outside
      `$HOME` each link back to the actual `$HOME/AGENTS.md`.
- [ ] Status, plan, setup, repair, and sync agree on the same effective target.
- [ ] The formerly fixed `../AGENTS.md` custom-home bridge is reported unhealthy
      and repairable rather than managed.
- [ ] Dangling, absolute, escaping, wrong-file, and non-symlink entries never
      pass health classification.
- [ ] Project root and nested Claude symlink/import representations retain
      their current documented placement and idempotence.

### Unit 3: Completed acknowledged repair semantics

**Story:** `epic-real-harness-recovery-filesystem-instructions-repair-outcome`

**Files:**

- `crates/cli/src/application/instructions.rs`
- `crates/cli/src/application/execution.rs`
- `crates/cli/tests/compiled_binary.rs`

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InstructionDisposition {
    unresolved_attention: bool,
}

fn finalize_instruction_result(
    outcome: &mut Outcome,
    report: &ExecutionReport,
    disposition: InstructionDisposition,
    postconditions_healthy: bool,
);
```

**Implementation notes:**

- Track unresolved conflicts separately from informational disclosures. Mark
  the disposition only when a requested scope remains blocked, unreadable, or
  unhealthy after the attempted plan.
- After an acknowledged divergent-file repair, verify that the recoverable
  backup is a regular file with the original bytes and that the selected
  bridge classifies as managed using Unit 2's specification.
- Complete when all executed operations are `Applied`/`NoChange`, there are no
  errors or unresolved scoped blockers, and postconditions are healthy. Do not
  leave the provisional document-load result or the repair disclosure as
  `attention_required`.
- Preserve exit 2 for unacknowledged divergence, backup failure, apply failure,
  mixed-scope partial completion, or failed re-observation. Plain and JSON
  outputs derive from the same `Outcome`.

**Acceptance:**

- [ ] `instructions repair --yes` and target-scoped `sync --yes` return
      `completed`/exit 0 after backup plus healthy replacement.
- [ ] Output still discloses that a recoverable backup was created without
      presenting a resolved decision as pending attention.
- [ ] Backup or postcondition failure returns attention-required with an
      actionable boundary reason.
- [ ] An unacknowledged divergent file remains untouched and exit 2.
- [ ] Repeating a successful repair reports `changed:false`, no new backup, and
      exit 0.

## Implementation order

1. `epic-real-harness-recovery-filesystem-instructions-executable-intent` and
   `epic-real-harness-recovery-filesystem-instructions-relative-bridges` may run
   in parallel; they own disjoint contracts except for coordinated exports from
   the domain module.
2. `epic-real-harness-recovery-filesystem-instructions-repair-outcome` follows
   relative-bridge health so finalization uses the single classifier.
3. Run focused core/runtime/storage/compiled-binary tests, then full workspace
   tests, formatting, strict Clippy, and the disposable-home reproduction.

## Testing

### Unit tests

- `crates/core/src/runtime/external_tree.rs`: executable and non-executable
  files are observed from verified descriptors; mode races fail closed.
- `crates/core/src/skill.rs`: whole-directory validation retains executable
  intent and mode-only fingerprint changes are unequal.
- `crates/core/src/storage/managed_artifact/tests/tree_contract.rs`: artifact
  equality includes executable intent and `Debug` redacts bytes.
- `crates/core/src/runtime/filesystem/directory_tree/tests.rs`: publication,
  load, rollback, and backups normalize modes to private executable/non-
  executable values.
- `crates/core/src/runtime/filesystem/tests/ownership.rs` and
  `crates/core/src/instructions.rs`: relative target computation/resolution
  covers sibling, ancestor, deep, root-escape, dangling, and wrong-target cases.

### Integration tests

- `crates/cli/tests/compiled_binary.rs`: install a skill containing `0755`,
  `0711`, `0644`, and shebang-only files to both harnesses at global and project
  scope; assert contents, normalized modes, state fingerprints, repeat no-op,
  and mode-only update behavior inside isolated roots.
- The same suite supplies custom `HOME`/`CODEX_HOME` layouts, asserts the
  effective bridge target, corrupts the bridge, repairs with acknowledgment,
  verifies original backup bytes, expects exit 0, and repeats with no change.
- Use only test-support-owned homes, config roots, workspaces, and fake native
  binaries. No test may inspect or mutate operator paths.

## Risks

- **Riskiest assumption:** all supported publication platforms expose POSIX
  execute bits. The implementation is already Unix-only at this boundary; if a
  future platform cannot represent owner execute, fail as unsupported rather
  than silently flattening intent.
- **Compatibility risk:** changing artifact equality/fingerprints means
  existing byte-identical installed trees may be reclassified once by mode.
  This is intended correction, not a schema migration; persisted fingerprints
  are refreshed only through normal observed mutation flows.
- **Security risk:** accepting arbitrary observed symlink spellings could hide
  root escape. Observation may normalize for comparison, but only an exact
  resolved canonical path with an existing regular destination is healthy;
  creation remains strict and relative.
- **Fallback:** if carrying intent in the general artifact contract proves too
  invasive, constrain `ArtifactFile` to the managed skill/publication pipeline,
  but never infer modes at destination or keep an unsynchronized side map.
