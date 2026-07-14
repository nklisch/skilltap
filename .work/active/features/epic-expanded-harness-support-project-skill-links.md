---
id: epic-expanded-harness-support-project-skill-links
kind: feature
stage: implementing
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-standalone-skill-lifecycle, epic-expanded-harness-support-registry]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Validate and Link Project-Local Skills

## Brief

Make one project-local portable skill tree authoritative across every selected
harness. The canonical complete skill directory remains
`<project>/.agents/skills/<name>/`; when a harness loads project skills from a
different native root, skilltap projects that skill as a per-skill relative
symlink back to the canonical directory. A harness that already consumes the
canonical location requires no projection.

Validate the canonical complete directory before planning links and report two
separate results: strict Agent Skills conformance and target-specific
loadability/compatibility. Validation covers the exact top-level `SKILL.md`,
portable frontmatter and name/directory invariants, complete-tree integrity,
and target-specific evidence already modeled by the standalone skill
lifecycle. A malformed or incompatible canonical skill is reported without
creating or repairing native links.

This feature extends the existing explicit project skill lifecycle; it does not
implicitly take ownership of every directory it can find. Skills become managed
through the existing install, adoption, or desired-inventory paths. Status,
plan, sync, update, and removal then observe and reconcile the canonical tree
and its selected target links as one resource.

## Strategic decisions

- **Canonical location:** `<project>/.agents/skills/<name>/` is the single
  project source of truth because it is the broadest portable convention and a
  native Codex load path.
- **Projection form:** use a relative symlink for each skill directory, not a
  copied tree and not a symlink for the whole harness skill root. Per-skill
  links preserve unmanaged and harness-specific sibling skills.
- **Selection and ownership:** preserve skilltap's explicit lifecycle. Merely
  observing an unmanaged canonical skill does not authorize mutation; adoption
  or installation establishes desired state and ownership.
- **Validation model:** keep strict format conformance distinct from observed
  target loadability. Client tolerance may produce a warning, but it never
  turns a nonconforming skill into a conforming one.
- **Conflict policy:** an existing regular file, directory, absolute symlink,
  or symlink to another target at a native destination is drift or an ownership
  conflict. `sync` does not overwrite it silently. A missing, broken, or
  incorrect skilltap-owned relative link is repairable through the normal plan
  and revalidated execution path.
- **Scope boundary:** this feature covers project scope. Global managed-skill
  representation remains unchanged unless separately scoped.

## Simplification opportunity

Stop publishing and fingerprinting duplicate complete trees in
`.agents/skills/` and harness-native project roots. Reuse the existing relative
symlink path logic, no-follow filesystem inspection, target registry, and
managed-skill execution boundary so canonical content validation, ownership,
and drift have one source of truth. Do not add a second project-skill registry,
manifest, or discovery command.

## Foundation references

- `docs/SPEC.md` — project scope, standalone skill model and lifecycle,
  compatibility, ownership, and symlink safety.
- `docs/ARCH.md` — standalone skill source of truth, adapter projection port,
  observation, and revalidated apply flow.
- `docs/HARNESS-CONTRACTS.md` — canonical `.agents/skills` placement, native
  project skill roots, whole-directory loading, and target compatibility.
- `.research/analysis/briefs/current-agent-extension-standards.md` — portable
  skill boundary and adapter projection posture.

The foundation already permits canonical `.agents/skills` trees and native
adapter links, while retaining copies for other scopes or independently scoped
fallbacks. This feature makes the project-scoped representation precise without
changing the standing product direction, so no foundation document is rolled
forward at scope time.

## Acceptance direction

- A valid managed project skill at `.agents/skills/<name>` produces one
  relative per-skill symlink in every selected distinct native skill root; the
  relative target resolves exactly to the canonical directory.
- Codex and any future harness whose native destination equals the canonical
  path produce no redundant operation.
- Correct links are immediate-repeat no-ops. Missing or skilltap-owned broken
  links are repaired; unmanaged or divergent destinations are reported and
  preserved.
- Install/update publishes and validates the canonical tree before dependent
  link operations. Remove deletes only proven skilltap-owned links and removes
  the canonical tree only when the selected resource removal is safe.
- Status and JSON output distinguish canonical format errors, target
  incompatibility, missing links, broken links, divergent links, and unmanaged
  destination conflicts without following a link for ownership decisions.
- Planning and application use the target registry rather than harness-id
  branching, revalidate link identity under the configuration lock, and remain
  idempotent on macOS and Linux.
- Isolated integration coverage exercises multiple harness roots, nested
  project paths, relative target calculation, complete skill siblings,
  conflicts, repair, removal, and an immediate repeated sync with zero changes.

## Dependency integration

This feature is the project-skill projection contract consumed by each pending
expanded-harness adapter family. Those features retain their native load-path
and effective-state responsibilities but do not invent their own copied-tree or
link reconciliation behavior.

## Grounding summary

The current implementation already has the right pieces, but combines them in
the wrong project representation:

- `ValidatedSkillTree` in `crates/core/src/skill.rs` validates a complete
  no-symlink tree and fingerprints file bytes plus executable intent, but it
  validates only top-level `SKILL.md` presence. It does not parse YAML or enforce
  the Agent Skills name, description, optional-field, or directory-name rules.
- `SkillCompatibility` in `crates/core/src/skill_compatibility.rs` uses a
  line-oriented frontmatter probe. It conflates strict conformance and
  loadability, accepts unterminated YAML as a warning, and returns the same
  result for every target.
- `SkillProjectionPort::destination` in
  `crates/harnesses/src/registry.rs` already gives the registry-driven native
  root. Codex resolves project skills to `.agents/skills`; Claude resolves them
  to `.claude/skills`. Comparing that root with the canonical root is enough to
  derive “no projection” versus “relative link”; no target-id branch is needed.
- `skill_destinations` and `execute_skill_install` in
  `crates/cli/src/application.rs` and
  `crates/cli/src/application/lifecycle.rs` currently publish a complete tree
  independently at every distinct destination through `ManagedSkillPort`.
  Project Codex therefore gets the canonical tree while project Claude gets a
  duplicate tree.
- `ManagedSkillPort` in `crates/cli/src/application/execution.rs` already binds
  tree requests to operation ids, revalidates affected surfaces under the
  configuration lock, and uses descriptor-relative tree publication/removal.
  It is retained for the one canonical tree.
- `ConfinedFileSystem` in
  `crates/core/src/runtime/filesystem/directory_tree.rs` supplies bounded
  descriptor-relative regular-file and tree operations, but has no final-entry
  symlink inspect/create/remove methods. The older absolute-path
  `FileSystem::create_relative_symlink` is suitable for instruction bridges,
  not for a managed descendant whose ancestors must also be no-follow.
- `TargetResourceState` and `ResourceState` in
  `crates/core/src/storage/state.rs` already preserve exact target-local
  provenance, ownership, fingerprints, revisions, and journals. No new state
  schema is needed: for project skills, each target fingerprint is the
  canonical tree fingerprint, while the expected link is deterministically
  derived from the project, adapter root, and skill name. A live link inode is
  ephemeral planning evidence and must not be persisted.
- `NativeObservation`/`StatusProjection` in
  `crates/cli/src/application/status.rs` currently report bounded native roots,
  not managed project skills and their canonical/link relationship. The
  unconditional `status_comparison_unavailable` warning confirms this missing
  semantic comparison.
- Existing compiled-binary coverage in
  `crates/cli/tests/compiled_binary.rs` deliberately asserts copied project
  trees in both `.agents/skills` and `.claude/skills`; those assertions become
  relative-link assertions for project scope only. Global behavior remains
  unchanged.

The design was mapped by direct reading only. The task is broad but the caller
explicitly prohibited nested agents and peers; the five stories below are
continuity and dependency checkpoints for one normal feature owner, not worker
fan-out targets.

## Design decisions

- **Project-only representation switch:** branch on `Scope::Project`, never on
  a harness id. Project scope publishes one canonical tree and derives each
  target projection from `SkillProjectionPort::destination`; global scope keeps
  the existing copy behavior. This honors the feature boundary and avoids a
  global migration hidden inside project work.
- **Strict YAML validation:** add `serde_yaml` and parse the frontmatter mapping
  once. Tree integrity errors and unparseable/missing required metadata are
  malformed and block canonical publication. Strict but potentially tolerated
  violations remain explicit conformance findings; they may proceed only when
  every selected adapter reports the exact shape loadable and foreground
  `--yes` acknowledges the nonconformance, preserving the parent lifecycle
  decision.
- **Adapter-owned loadability:** extend `SkillProjectionPort` with a default
  compatibility method returning the conservative portable result from core.
  Target adapters override it when attested fields, variables, or load behavior
  differ. The CLI never reinterprets target ids or native frontmatter.
- **Unknown extension fields:** preserve the complete source tree and unknown
  YAML fields. Unknown top-level fields are non-portable evidence, not deleted
  or rewritten; absent adapter evidence yields `CompatibilityClass::Unknown`,
  not a false compatible claim.
- **No persisted link manifest:** expected project link destination and relative
  target are pure functions of `ResourceKey`, `AgentSkillName`, project root,
  and the selected adapter's projection root. Persisting a second manifest would
  duplicate the typed registry and create path-migration state. Existing
  target-local ownership plus a planning-time no-follow `LinkIdentity` is the
  authority for repair/removal.
- **Target isolation for shared content:** a project canonical tree is consumed
  by every desired target, so replacing or recreating that tree cannot honor a
  target subset. `skill update`, `sync`, or repair that changes canonical bytes
  is blocked unless the selected targets cover every desired target binding for
  that resource. Link-only repair/removal remains target-selectable. This is
  preferred over silently changing an unselected harness or reintroducing
  per-target copies.
- **Install can add targets:** on an existing project resource, `skill install
  ... --target X` unions `X` into the desired target set when the validated
  source fingerprint equals the canonical tree. It creates only the missing
  target link. A different source fingerprint still requires the explicit
  update path and the all-desired-target rule.
- **Adoption does not transfer canonical deletion authority:** explicit project
  adoption or a source-less desired entry may use a valid existing canonical
  tree as the managed source and create selected target links. A canonical path
  directly consumed by a harness remains `Provenance::Adopted` /
  `Ownership::Harness`; skilltap-created links are target-local direct
  projections. Removing the last adopted target removes owned links but
  preserves the adopted canonical tree, which returns to unmanaged status.
- **Owned relative-link repair only:** missing links are creatable after desired
  ownership exists. A divergent *relative* symlink is replaceable only when the
  exact target binding is skilltap-owned; replacement captures and revalidates
  the current inode and target under lock. Regular files, directories, special
  entries, absolute symlinks, and any divergent/untracked symlink are conflicts
  and are preserved.
- **Dependency order is load-safety order:** install/update canonical tree first,
  then target links. Removal reverses it: remove every selected owned link, then
  remove the canonical tree only when no target remains and all required link
  removals succeeded. `OperationDependency` encodes both orders so failures skip
  dependents.
- **No UI work:** this is a non-interactive CLI/domain/filesystem feature; no
  screen or flow surface exists, so the UI fallback is skipped.

## Architectural choice

**Chosen — pure project-skill contract plus confined link executor.** Core owns
strict format/loadability result types, project layout derivation, projection
health, and pure planning decisions. The registry's existing
`SkillProjectionPort` supplies native roots and gains adapter-owned
compatibility analysis. The CLI application composes source/inventory/state
with those pure decisions. `ManagedSkillPort` continues to publish the single
canonical tree; a sibling `ProjectSkillLinkPort` performs descriptor-relative
link mutation. A composite execution port binds both request maps to one
validated dependency graph and one configuration lock.

**Rejected — minimally change `skill_destinations` to call
`FileSystem::create_relative_symlink`.** This is short but leaves strict
validation, status, adoption, ownership, ancestor-symlink escape, plan/apply
races, and dependency ordering unresolved. It would create links safely only in
the happy path.

**Rejected — let every adapter reconcile its own project links.** Adapters own
native roots and compatibility evidence, not shared ownership or execution
semantics. Moving reconciliation into each adapter would duplicate drift,
repair, removal, rollback, output, and idempotency logic across every expanded
harness.

**Rejected — copy canonical content to each target and compare fingerprints.**
This is current behavior. It preserves target-isolated versions but directly
contradicts the feature's one-authoritative-project-tree decision and creates
three drift surfaces for one logical resource.

## Implementation Units

### Unit 1: Strict Agent Skills validation and project layout contract

**Files**:

- `Cargo.toml` and `crates/core/Cargo.toml` — add workspace `serde_yaml`.
- `crates/core/src/skill_compatibility.rs` — replace the line parser with
  strict metadata/conformance and separate target loadability.
- `crates/core/src/project_skill.rs` (new) — derive canonical paths, target
  projections, link health, and pure lifecycle decisions.
- `crates/core/src/lib.rs` — export the project-skill contract.

**Story**: `epic-expanded-harness-support-project-skill-links-contract`

```rust
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AgentSkillName(String);

impl AgentSkillName {
    pub fn new(value: impl Into<String>) -> Result<Self, AgentSkillNameError>;
    pub fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillMetadata {
    pub name: AgentSkillName,
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub allowed_tools: Option<String>,
    pub extension_fields: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSkillConformance {
    Conforming,
    Nonconforming,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentSkillFormatFinding {
    InvalidUtf8,
    MissingFrontmatter,
    UnterminatedFrontmatter,
    InvalidYaml,
    FrontmatterNotMapping,
    MissingName,
    InvalidName,
    DirectoryNameMismatch,
    MissingDescription,
    DescriptionTooLong,
    InvalidLicense,
    InvalidCompatibility,
    InvalidMetadata,
    InvalidAllowedTools,
    ExtensionField(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillValidation {
    metadata: Option<AgentSkillMetadata>,
    conformance: AgentSkillConformance,
    loadable_shape: bool,
    findings: Vec<AgentSkillFormatFinding>,
}

pub fn validate_agent_skill(
    tree: &ValidatedSkillTree,
    directory_name: &AgentSkillName,
) -> AgentSkillValidation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillLoadability {
    Loadable,
    Unknown,
    Blocked,
}

pub struct SkillCompatibility {
    target: HarnessId,
    class: CompatibilityClass,
    loadability: SkillLoadability,
    findings: Vec<SkillCompatibilityFinding>,
}

impl SkillCompatibility {
    pub fn portable(target: HarnessId, validation: &AgentSkillValidation) -> Self;
    pub fn target(&self) -> &HarnessId;
    pub const fn class(&self) -> CompatibilityClass;
    pub const fn loadability(&self) -> SkillLoadability;
    pub fn findings(&self) -> &[SkillCompatibilityFinding];
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetProjectSkillProjection {
    Canonical { path: AbsolutePath },
    RelativeLink(ProjectSkillLinkSpec),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSkillLinkSpec {
    pub project_root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub canonical_path: AbsolutePath,
    pub target: RelativeSymlinkTarget,
}

pub fn project_skill_projection(
    project: &AbsolutePath,
    native_skill_root: &AbsolutePath,
    name: &AgentSkillName,
) -> Result<TargetProjectSkillProjection, ProjectSkillPathError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSkillLinkHealth {
    NotRequired,
    Healthy,
    Missing,
    Broken,
    Divergent,
    UnmanagedConflict,
}
```

**Implementation notes**:

- Parse only the bounded `SKILL.md` bytes already captured by
  `ValidatedSkillTree`; never reopen a source path after snapshot validation.
- `AgentSkillName` exactly enforces the normative one-to-64 lowercase ASCII
  letter/digit/hyphen grammar, edge/consecutive-hyphen restrictions, and is
  serde-validating if it crosses a wire. The canonical directory name and
  frontmatter `name` must compare equal.
- Portable field types and limits come from the attested Agent Skills contract:
  `description` 1–1024 chars, `compatibility` 1–500 chars, `metadata` string to
  string, and `allowed-tools` a string. Unknown fields remain in
  `extension_fields` and make strict portable conformance false; nothing is
  rewritten or dropped.
- `loadable_shape` is false for invalid YAML or absent required metadata. A
  nonconforming but parseable shape is not automatically loadable; adapters
  must provide evidence. The default portable compatibility is `Compatible`
  only for conforming metadata, `Unknown` for parseable nonconformance, and
  `Incompatible`/`Blocked` for malformed metadata.
- `project_skill_projection` requires both roots beneath the canonical project,
  normalizes a project-relative destination, compares complete destination
  paths, and computes the lexical relative target from the link parent. Equal
  paths produce `Canonical` and therefore no link operation.

**Acceptance criteria**:

- [ ] A complete tree with valid YAML, exact required/optional field types,
      conforming name, and matching directory is `Conforming`.
- [ ] Missing exact top-level `SKILL.md`, an internal symlink, invalid YAML,
      missing required metadata, or a directory/name mismatch is represented by
      the exact tree/format finding and never misreported as conforming.
- [ ] Strict conformance and target compatibility/loadability are independently
      queryable; one cannot overwrite the other.
- [ ] Project roots equal to `.agents/skills` produce `Canonical`; `.claude/skills`
      and arbitrary future project descendants produce normalized relative-link
      specs with no harness-id branches.
- [ ] A native project root outside the project or through an invalid relative
      component is rejected before planning.

---

### Unit 2: Descriptor-relative project symlink boundary

**Files**:

- `crates/core/src/runtime/filesystem/directory_tree.rs` and its
  `tree_io.rs`/`unix_support.rs` helpers — confined final-entry link operations.
- `crates/core/src/runtime/error.rs` and `crates/core/src/runtime/mod.rs` — link
  identity and observation exports.
- `crates/core/src/runtime/filesystem/directory_tree/tests.rs` — race,
  confinement, and no-follow tests.
- `crates/harnesses/src/registry.rs`,
  `crates/harnesses/src/adapters/codex.rs`, and
  `crates/harnesses/src/adapters/claude.rs` — adapter compatibility method and
  project-root contract tests.

**Story**: `epic-expanded-harness-support-project-skill-links-filesystem`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinkIdentity {
    device: u64,
    inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfinedEntryObservation {
    Missing,
    RelativeSymlink {
        identity: LinkIdentity,
        target: RelativeSymlinkTarget,
    },
    AbsoluteSymlink { identity: LinkIdentity },
    RegularFile,
    Directory,
    Other,
}

pub trait ConfinedFileSystem {
    // existing methods remain
    fn inspect_entry_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<ConfinedEntryObservation, RuntimeError>;

    fn create_relative_symlink_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        target: &RelativeSymlinkTarget,
    ) -> Result<LinkIdentity, RuntimeError>;

    fn remove_relative_symlink_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        expected_identity: LinkIdentity,
        expected_target: &RelativeSymlinkTarget,
    ) -> Result<LinkIdentity, RuntimeError>;
}

pub trait SkillProjectionPort: Sync {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath>;

    fn compatibility(
        &self,
        target: &HarnessId,
        skill: &ValidatedSkillTree,
        validation: &AgentSkillValidation,
    ) -> SkillCompatibility {
        SkillCompatibility::portable(target.clone(), validation)
    }
}
```

**Implementation notes**:

- Every ancestor is opened descriptor-relative with `O_NOFOLLOW`; the final
  entry is inspected with `fstatat(..., AT_SYMLINK_NOFOLLOW)` and read with a
  bounded `readlinkat`. Absolute and malformed targets are classified, never
  normalized into managed evidence.
- Creation makes only missing directory ancestors beneath the already canonical
  project root, rejects symlink/non-directory ancestors, creates the final link
  with `symlinkat`, captures its inode, and syncs the parent directory.
- Removal verifies both inode and lexical target immediately before `unlinkat`.
  It never follows the link and never removes a directory or regular file.
- `LinkIdentity` is process-local race evidence like `DirectoryIdentity`; it is
  intentionally not serializable or persisted.
- The compatibility method lives on the existing skill port rather than adding
  another optional adapter port. Codex and Claude initially use the conservative
  default; expanded adapters override only with attested native evidence.

**Acceptance criteria**:

- [ ] Missing, relative symlink, absolute symlink, regular file, directory, and
      special entry are distinguishable without following the final component.
- [ ] A symlinked ancestor, changed inode, changed target, path escape, oversized
      target, or non-directory ancestor fails without mutating the outside path.
- [ ] Create/remove are durable and idempotent at their public contract; stale
      identity removal never deletes a replacement.
- [ ] Codex and Claude project roots are still registry-owned and their default
      compatibility result is conservative.

---

### Unit 3: Dependency-ordered project skill lifecycle

**Files**:

- `crates/cli/src/application/project_skills.rs` (new) — project source,
  desired/state, observation, and pure-plan composition.
- `crates/cli/src/application.rs` — module wiring; retire project handling from
  `skill_destinations` while retaining the global helper.
- `crates/cli/src/application/lifecycle.rs` and
  `crates/cli/src/application/reconciliation.rs` — route project
  install/update/remove/plan/sync through the shared project service.
- `crates/cli/src/application/execution.rs` — link request and composite
  execution ports.
- `crates/core/src/lifecycle_operation.rs` — use the existing dependency-aware
  faithful operation constructor; only add a link-specific reason helper if it
  removes repeated authored evidence.

**Story**: `epic-expanded-harness-support-project-skill-links-lifecycle`

```rust
pub(super) struct ProjectSkillLinkEntry {
    pub root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub target: RelativeSymlinkTarget,
    pub action: ProjectSkillLinkAction,
}

pub(super) enum ProjectSkillLinkAction {
    Create,
    Replace {
        expected_identity: LinkIdentity,
        previous_target: RelativeSymlinkTarget,
    },
    Remove {
        expected_identity: LinkIdentity,
    },
}

pub(super) struct ProjectSkillLinkPort<'a> {
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub entries: BTreeMap<OperationId, ProjectSkillLinkEntry>,
    pub foreign_operations: BTreeSet<OperationId>,
}

pub(super) struct ProjectSkillLifecyclePort<'a> {
    pub canonical: ManagedSkillPort<'a>,
    pub links: ProjectSkillLinkPort<'a>,
}

impl ExecutionPort for ProjectSkillLifecyclePort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError>;
    fn apply(&self, operation: &Operation) -> Result<OperationOutcome, ExecutionError>;
}

pub(super) struct ProjectSkillPlan {
    pub operations: Vec<Operation>,
    pub canonical_entries: BTreeMap<OperationId, ManagedSkillEntry>,
    pub link_entries: BTreeMap<OperationId, ProjectSkillLinkEntry>,
    pub state_seeds: BTreeMap<ResourceKey, ResourceState>,
    pub affected_targets: HarnessSet,
}

fn plan_project_skill_lifecycle(
    context: ProjectSkillPlanContext<'_>,
) -> Result<ProjectSkillPlan, ProjectSkillPlanError>;
```

**Implementation notes**:

- Resolve and snapshot an install/update source once, validate tree integrity and
  metadata once, then ask each selected adapter for target compatibility. No
  inventory, state, canonical tree, or native link mutation occurs when the
  canonical shape is malformed or a required target is incompatible.
- The canonical operation uses `ManagedSkillPort` at
  `<project>/.agents` + `skills/<name>`. Each link operation uses the project
  root plus a project-relative destination. A link operation depends on the
  canonical publish/replace operation when one exists.
- Correct canonical content and links emit no mutation. An owned missing link is
  created. An owned divergent relative link is replaced under identity
  revalidation; if creation fails after unlink, restore the captured previous
  relative link only while the path is still absent, then reobserve and report
  residual state if restoration cannot be proven.
- Source publication still precedes projection. A failed canonical operation
  causes the executor to skip dependent links. Independent target links may
  proceed only when their canonical prerequisite is already healthy.
- Project updates that change or recreate canonical content require
  `selected_targets == desired_targets`; otherwise return
  `project_skill_shared_content_requires_all_targets` with a `--target all`
  next action. Link-only plans do not use this gate.
- Existing desired targets and explicitly requested install targets are unioned.
  Existing source changes on `skill install` remain `skill_update_required`.
- Every project target seed carries the same canonical fingerprint/revision but
  preserves its own native id, provenance, ownership, available revision, and
  operation journal. `refresh_resource_state` updates selected/actually affected
  bindings and preserves unselected sibling evidence.
- For removal, classify every selected projection before planning. Any unmanaged
  conflict blocks the resource removal without pruning inventory/state. Owned
  links are removed first. The canonical remove operation depends on all link
  removals and exists only when no desired target remains and the canonical tree
  was installed by skilltap. Adopted canonical trees are preserved.
- The composite port follows the established foreign-operation pattern used by
  native/managed hybrid lifecycle: each child port proves every request it owns,
  accepts only explicitly declared foreign ids, and dispatches apply by exact
  operation id.

**Acceptance criteria**:

- [ ] A project install publishes exactly one complete canonical tree and one
      relative per-skill link per distinct noncanonical target root.
- [ ] Canonical-path targets produce no redundant link/copy operation; duplicate
      target roots collapse to one physical link while retaining target-local
      state bindings.
- [ ] Canonical publish/replace precedes links; link removals precede canonical
      removal; failed operations skip only their declared dependents.
- [ ] Repeated install/update/sync is a zero-change plan and does not rewrite
      link inodes.
- [ ] Targeted link repair/removal preserves unselected bindings. Canonical-byte
      updates cannot silently affect an unselected desired target.
- [ ] Regular directories/files, absolute links, special entries, and unmanaged
      divergent links are preserved and reported as conflicts.
- [ ] A replace/remove race is rejected under lock and never removes the
      replacement.

---

### Unit 4: Semantic observation, status, adoption, and sync

**Files**:

- `crates/cli/src/application/project_skills.rs` — bounded canonical-root
  enumeration and desired-resource observation.
- `crates/cli/src/application/status.rs` — render project skill status and feed
  canonical adoption candidates into normalized observation.
- `crates/core/src/domain/resource/finding.rs` — register project-skill format,
  compatibility, link, and destination-conflict finding vocabularies.
- `crates/core/src/adoption.rs` only if needed to preserve an adopted canonical
  skill's source-less desired representation; do not add a separate adoption
  path or state document.
- `crates/cli/src/application/reconciliation.rs` — source-less desired project
  skills reconcile from their validated canonical tree.

**Story**: `epic-expanded-harness-support-project-skill-links-observation`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSkillObservation {
    pub resource: ResourceKey,
    pub canonical: CanonicalProjectSkillObservation,
    pub targets: BTreeMap<HarnessId, TargetProjectSkillObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalProjectSkillObservation {
    Missing,
    Invalid {
        tree_error: Option<SkillTreeError>,
        format: Option<AgentSkillValidation>,
    },
    Present {
        fingerprint: Fingerprint,
        format: AgentSkillValidation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetProjectSkillObservation {
    pub compatibility: SkillCompatibility,
    pub projection: ProjectSkillLinkHealth,
    pub ownership: Ownership,
}

fn observe_project_skill(
    registry: &TargetRegistry,
    filesystem: &dyn ProjectSkillFileSystem,
    paths: &PlatformPaths,
    resource: &DesiredResource,
    state: Option<&ResourceState>,
    selected_targets: &HarnessSet,
    limits: ExternalTreeLimits,
) -> Result<ProjectSkillObservation, ProjectSkillObservationError>;
```

**Implementation notes**:

- Desired project resources are observed by exact name. For status/adoption of
  unmanaged canonical skills, enumerate only direct children of
  `.agents/skills` within bounded entry/depth/byte limits; do not recursively
  search repositories or marketplace content.
- Inspect native destinations without following links for ownership. Resolve an
  observed relative target lexically and inspect the canonical tree separately
  for health; never follow the native link to fingerprint content.
- Register and render stable codes for `skill.format.invalid`,
  `skill.target.incompatible`, `skill.link.missing`, `skill.link.broken`,
  `skill.link.divergent`, and `skill.destination.unmanaged`. JSON resource
  entries carry separate `conformance`, `compatibility`, `loadability`, and
  `projection` fields. Plain output is derived from the same outcome.
- A healthy canonical-path target renders `projection=not_required`; a healthy
  distinct root renders `projection=linked`. Missing, broken, divergent, and
  conflict states remain distinct in both output forms.
- Replace the blanket `status_comparison_unavailable` warning only for project
  standalone skills that now receive semantic comparison; retain it for any
  still-uncompared resource kinds.
- Canonical direct-child observations become adoption candidates only through
  the existing `adopt --project` selection. Observation alone stays unmanaged.
  Adoption writes desired inventory only, as the standing adoption contract
  requires. A later plan/sync validates the same canonical fingerprint under
  lock, seeds target state, and creates only selected links.
- A source-less desired project skill is valid only when its canonical tree is
  present and validates. If it disappears or changes before apply, revalidation
  blocks; no link is created. It has no remote update lifecycle.
- Native regular directories in noncanonical roots remain status conflicts, not
  implicit adoption sources. This avoids silently moving or deleting a harness-
  specific tree while still allowing the canonical portable tree to be adopted.

**Acceptance criteria**:

- [ ] Status and JSON independently expose strict conformance, target
      compatibility/loadability, and projection health for every selected
      managed project skill.
- [ ] Missing canonical, malformed canonical, target incompatibility, missing
      link, broken link, divergent link, and unmanaged destination conflict
      produce distinct stable codes and actionable next steps.
- [ ] `status` never mutates and never follows a native link for ownership or
      fingerprint decisions.
- [ ] Explicit adoption/source-less desired state can reconcile a valid
      canonical tree without copying it or claiming unrelated native
      destinations.
- [ ] Incompatible/malformed canonical content prevents link creation/repair;
      healthy siblings remain observable.

---

### Unit 5: Isolated lifecycle and output acceptance coverage

**Files**:

- `crates/cli/tests/compiled_binary.rs` — project install/update/remove/status,
  JSON, conflict, repair, and immediate-repeat cases.
- `crates/core/src/runtime/filesystem/directory_tree/tests.rs` — low-level
  no-follow and race cases from Unit 2.
- `crates/core/src/skill_compatibility.rs` and
  `crates/core/src/project_skill.rs` unit tests — strict metadata and pure
  layout/health decisions.
- `crates/test-support/src/integration.rs` — add a project-root no-follow
  snapshot helper only if the compiled tests cannot express link identity and
  target assertions clearly with the existing `snapshot_tree`.

**Story**: `epic-expanded-harness-support-project-skill-links-acceptance`

**Implementation notes**:

- Update only project-scope copy assertions. Existing global canonical plus
  Claude-copy tests remain unchanged and guard the explicit scope boundary.
- Use `IsolatedMachine` roots and fake harness profiles; never inspect the
  operator's real project, HOME, Codex, or Claude state.
- Snapshot links without following them. Verify complete siblings by resolving
  the link only in the test assertion after first proving the native entry is a
  relative symlink with the expected lexical target.

**Acceptance criteria**:

- [ ] A nested project path produces the correct relative target for Codex plus
      Claude and for a throwaway registry adapter with another native root.
- [ ] Complete `SKILL.md`, script, reference, asset, executable intent, and
      unknown sibling files remain present only in the canonical tree and are
      reachable through each correct link.
- [ ] Correct repeat is a no-op; missing link repairs; broken link repairs by
      restoring canonical content first; owned divergent relative link repairs;
      unmanaged/absolute/file/directory conflicts remain byte-for-byte intact.
- [ ] Targeted removal preserves canonical content while another target remains;
      final direct removal deletes owned links before canonical; final adopted
      removal preserves canonical.
- [ ] A partial-target content update is blocked, while all-target update changes
      canonical content once and every link immediately observes it.
- [ ] Plain and JSON outcomes carry the same status distinctions and exit 0/2/3
      according to completed, attention, and partial-apply contracts.
- [ ] Every mutating scenario immediately repeats and reports zero changes.
- [ ] `cargo test --workspace --all-targets`, clippy with warnings denied,
      formatting, and `git diff --check` pass before feature review.

## Implementation Order

1. `epic-expanded-harness-support-project-skill-links-contract` — Unit 1,
   `depends_on: []`.
2. `epic-expanded-harness-support-project-skill-links-filesystem` — Unit 2,
   `depends_on: [epic-expanded-harness-support-project-skill-links-contract]`.
3. `epic-expanded-harness-support-project-skill-links-lifecycle` — Unit 3,
   `depends_on: [epic-expanded-harness-support-project-skill-links-contract,
   epic-expanded-harness-support-project-skill-links-filesystem]`.
4. `epic-expanded-harness-support-project-skill-links-observation` — Unit 4,
   `depends_on: [epic-expanded-harness-support-project-skill-links-contract,
   epic-expanded-harness-support-project-skill-links-lifecycle]`.
5. `epic-expanded-harness-support-project-skill-links-acceptance` — Unit 5,
   `depends_on: [epic-expanded-harness-support-project-skill-links-lifecycle,
   epic-expanded-harness-support-project-skill-links-observation]`.

`work-view --blocking <story-id>` was run for every story receiving a sibling
`depends_on` entry before these edges were written; all returned no existing
dependents, so the graph introduces no cycle. These stories are design
checkpoints for one cohesive implementation owner. They are not five parallel
worker assignments: Units 1–2 establish contracts, Unit 3 owns the mutation
slice, Unit 4 consumes it for observation, and Unit 5 closes integrated
evidence.

## Simplification

- Replace project `skill_destinations` duplicate-tree expansion with one
  `project_skill_projection` derivation. Keep the existing global helper rather
  than forcing one abstraction across different representations.
- Delete the line-oriented frontmatter parser and its duplicate
  `strict_agent_skills` boolean; one YAML validation result becomes the source
  for strict conformance and adapter compatibility input.
- Reuse `ManagedSkillPort` for the canonical tree and
  `faithful_file_operation_with_dependencies` for ordering; do not create a
  second tree publisher or operation graph.
- Reuse `SkillProjectionPort` for both path and target loadability evidence; do
  not add a parallel project-skill adapter registry.
- Do not persist link paths, targets, or inode identities. Deterministic layout
  plus target-local ownership and fresh no-follow observation are sufficient.
- Consolidate project skill orchestration in
  `application/project_skills.rs`, reducing the large standalone lifecycle
  block without changing unrelated marketplace/plugin lifecycle.
- Remove project tests that merely assert duplicate bytes in two trees; replace
  them with one canonical-integrity assertion and one link-contract assertion.
  Retain global copy tests and independent source snapshot tests because they
  protect different guarantees.

No separate cleanup/refactor story is warranted. Each deletion is coupled to
its replacement and stays inside the checkpoint that can verify it.

## Testing

- **Format interface tests:** YAML parsing, exact portable field limits/types,
  name grammar and directory equality, extension-field preservation, malformed
  blocking, and strict-versus-loadability separation. Protects the public skill
  contract rather than parser branches.
- **Layout/property tests:** canonical/native roots at varying project depths
  always produce a normalized relative target that lexically resolves to the
  canonical path; roots outside the project reject. Protects platform-neutral
  path arithmetic.
- **Filesystem regression tests:** ancestor symlinks, final-entry kinds,
  oversized/malformed targets, stale inode replacement, and create/remove
  durability. Protects the root-confined ownership boundary.
- **Planner tests:** canonical-before-link, link-before-canonical-remove,
  duplicate-root collapse, partial-target update block, conflict preservation,
  and dependency failure propagation. Protects the cross-unit semantics.
- **Compiled CLI tests:** complete project lifecycle, target-local remove/repair,
  adopted canonical behavior, status/JSON parity, exit codes, and immediate
  repeats. Protects the user-visible contract.
- **Test removal/update:** project copied-tree assertions become link assertions;
  the global copied-tree and target-local global-version tests stay because this
  feature intentionally does not change global representation.

No test is added for trivial getters, static operation-id formatting, or every
YAML parser error string. Stable enums/codes, operation ordering, ownership
safety, and externally visible behavior are the earned surfaces.

## Risks

- **Riskiest assumption — shared canonical content versus `--target`:** one tree
  cannot hold two target-specific versions. Silently updating an unselected
  harness would violate target selection; copying per target would violate this
  feature. The all-desired-target gate is the explicit, reversible resolution.
  If product direction later permits resource-wide project updates regardless
  of `--target`, only this gate and output change; the storage/link design does
  not.
- **Repair ownership ambiguity:** persisted state proves logical ownership, but
  a pathname can be replaced between observation and apply. The link port uses
  live inode plus target evidence under lock and never treats a regular file,
  directory, absolute link, or unmanaged relative link as repairable. Failed
  replacement restores only the captured owned relative representation.
- **YAML parser behavior:** adding `serde_yaml` increases dependency surface and
  YAML has complex features. Input is already byte-bounded; the parser result is
  converted immediately into a small owned contract and raw YAML never enters
  state/output. If dependency maintenance becomes unacceptable, replace the
  parser behind `validate_agent_skill` without changing callers.
- **Adopted canonical disappearance:** a source-less adopted tree cannot be
  recreated. Status reports missing/broken and sync blocks without changing
  links; the fallback is reinstall from an explicit source, not guessing one.
- **Adapter evidence lag:** new harnesses may initially use conservative
  `Unknown` compatibility. This blocks mutation rather than overclaiming
  portability. Each adapter feature can override the compatibility method when
  its native contract is attested.
- **macOS/Linux filesystem differences:** descriptor-relative APIs and symlink
  durability differ at the syscall edge. Unit 2 keeps platform code behind the
  existing Unix runtime boundary and the compiled matrix must run on both
  supported CI platforms.

## Other agent review

- **Effective review weight:** standard (caller/default).
- **Design-time advisory decision:** warranted by the cross-harness ownership,
  link-race, and target-isolation contracts, but skipped because the delegated
  endpoint explicitly forbids subagents and peeragent. Design review is
  non-blocking under the workflow policy.
- **Implementation closure:** the feature still requires one independent
  standard-weight feature review after all child checkpoints verify. Child
  stories advance directly to done on green evidence and do not become review
  units.
