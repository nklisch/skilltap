---
id: feature-split-status-application
kind: feature
stage: done
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Split the StatusApplication god module

## Brief

Split the 6.8k-line `StatusApplication` implementation in
`crates/cli/src/application.rs` into private responsibility modules without
changing public signatures, output schemas, command behavior, or ownership
boundaries. The current module combines execution ports, daemon cycles,
marketplace/plugin lifecycle, skills, instructions, reconciliation,
adoption/status, and helper functions.

## Refactor constraints

- Pure behavior-preserving extraction only; no API or output changes.
- Keep composition and dependency wiring explicit at the CLI boundary.
- Preserve all existing tests and add no compatibility layer.
- Candidate boundaries: execution ports; lifecycle/skills; instructions and
  reconciliation; status/adoption.

## Acceptance

- Each extracted module has a coherent private responsibility.
- Existing workspace tests, formatting, and clippy remain green.
- Public command signatures and output remain byte/schema compatible.

## Design decisions

- Keep `crates/cli/src/application.rs` as the composition-facing module and add
  private child modules under `crates/cli/src/application/`. This avoids a
  public API or `entrypoint.rs` wiring change while giving each responsibility
  an independently reviewable implementation unit.
- Use `impl StatusApplication<'_>` blocks in the child modules. The methods
  called by `entrypoint.rs` retain their existing `pub(crate)` signatures;
  intra-application helpers remain private or `pub(super)` only when another
  child module or the existing `application/tests.rs` needs them.
- Keep all outcome construction, error text, operation IDs, native argument
  vectors, lock/journal wiring, and filesystem boundaries unchanged. This is a
  structural extraction, not an opportunity to deduplicate behavior or alter
  the `--yes` contract.
- Keep the existing `application/tests.rs` location. The parent module will
  re-export only the two tested helpers moved into children
  (`lifecycle_operation_id` and `normalize_daemon_noop_result`) with
  `pub(super) use`, so test call sites do not change.
- The lifecycle and instruction modules may proceed independently after the
  execution-port extraction. Reconciliation depends on both because it calls
  their existing adapters; status/adoption is last because its observation
  projection shares document and scope helpers with reconciliation.

## Refactor Overview

`application.rs` is currently 6,823 lines and combines five distinct concerns:
execution ports and state journaling (lines 101–630), native marketplace/plugin
and standalone-skill lifecycle (lines 633–3,030 plus helpers through 5,560),
instruction bridge setup/status (lines 918–3,780 plus bridge helpers), plan and
sync orchestration (lines 633–840 and 3,780–4,290), and status/adoption/native
observation (lines 4,290–6,823). This makes changes to one adapter difficult to
review and forces every command to scan a god module.

The target is a private module graph with one narrow application façade:

```text
crates/cli/src/application.rs       StatusApplication, shared imports/types,
                                    module declarations, stable re-exports
├── application/execution.rs        StateExecutionJournal + execution ports
├── application/lifecycle.rs        daemon/update, native lifecycle, skills
├── application/instructions.rs     instruction status/setup + bridge helpers
├── application/reconciliation.rs   plan/sync candidate projection + helpers
└── application/status.rs            status/adoption + observation/document scope
```

`entrypoint.rs` continues to construct `StatusApplication` exactly as it does
today (lines 883–916) and continues calling the same `pub(crate)` methods.
There is no new trait, dependency direction, output type, command argument, or
compatibility shim.

## Refactor Steps

### Step 1: Extract execution ports and the state journal

**Priority**: High  
**Risk**: Medium  
**Source Lens**: code smell / missing abstraction (mixed side-effect ports in a command façade)  
**Files**: `crates/cli/src/application.rs`, new `crates/cli/src/application/execution.rs`, `crates/cli/src/application/tests.rs` (only if module imports require a test visibility adjustment)  
**Story**: `story-split-status-application-execution-ports`

**Current State**:

`StateExecutionJournal`, `ManagedSkillPort`, `ManagedSkillEntry`,
`ManagedSkillAction`, `InstructionPort`, `InstructionEntry`,
`InstructionWrite`, and both `ExecutionPort` implementations are top-level
private items in `application.rs` (lines 101–630). Lifecycle and instruction
methods instantiate these concrete ports directly.

**Target State**:

```rust
// application/execution.rs
pub(super) struct StateExecutionJournal<'a> { /* same fields */ }
impl ExecutionJournal for StateExecutionJournal<'_> { /* byte-identical body */ }

pub(super) struct ManagedSkillPort<'a> { /* same fields */ }
impl ExecutionPort for ManagedSkillPort<'_> { /* byte-identical body */ }

pub(super) struct InstructionPort<'a> { /* same fields */ }
impl ExecutionPort for InstructionPort<'_> { /* byte-identical body */ }
```

`application.rs` declares `mod execution;` and imports these types with
`use execution::{InstructionPort, ManagedSkillPort, StateExecutionJournal};`.
Failure constructors remain in this module or are `pub(super)` from
`execution.rs`; their evidence codes and messages do not change.

**Implementation Notes**:

- Move the associated entry/action enums and helper constructors with the port
  that owns them; do not create a generic filesystem port.
- Preserve `BTreeMap` keying, operation-surface revalidation, backup/rollback
  order, and state replacement semantics exactly.
- Keep `StateExecutionJournal` visible to lifecycle, skill, and instruction
  children through `pub(super)`; it remains unavailable outside `application`.
- Run the existing unit and integration tests immediately after extraction to
  catch import/visibility regressions before moving behavioral methods.

**Acceptance Criteria**:

- [ ] `application/execution.rs` owns all three execution-port implementations
  and no duplicate definitions remain in `application.rs`.
- [ ] `cargo test -p skilltap-cli --offline` passes without changing a test
  assertion or rendered output.
- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --offline -- -D warnings` pass.
- [ ] `entrypoint.rs` and all port call sites compile with the same signatures.

**Risk**: Private visibility or lifetime/import mistakes could cause a compile
failure; moving port code must not alter side-effect order.  
**Rollback**: Revert the extraction commit and restore the three port blocks to
`application.rs`; no persisted state or native files are changed by this step.

---

### Step 2: Extract native lifecycle and standalone-skill flows

**Priority**: High  
**Risk**: High  
**Source Lens**: code smell / god module (marketplace, plugin, skill, and update responsibilities)  
**Files**: `crates/cli/src/application.rs`, new `crates/cli/src/application/lifecycle.rs`, `crates/cli/src/application/execution.rs` (imports only)  
**Story**: `story-split-status-application-lifecycle`  
**Depends On**: `story-split-status-application-execution-ports`

**Current State**:

The single `StatusApplication` impl contains daemon update orchestration,
`execute_lifecycle_preview`, `execute_native_lifecycle`,
`execute_skill_install`, `execute_skill_update`, and `execute_skill_remove`
(lines 633–3,030). `NativeLifecycleSpec`, `SkillDestination`, Git source
resolution, operation-ID helpers, inventory/state projection, and native
presence helpers are also top-level in the same file (lines 4,586–5,560).

**Target State**:

```rust
// application/lifecycle.rs
impl StatusApplication<'_> {
    pub(crate) fn execute_daemon_cycle(&self) -> Outcome;
    pub(crate) fn execute_lifecycle_preview(...same parameters...) -> Outcome;
    pub(crate) fn execute_native_lifecycle(...same parameters...) -> Outcome;
    pub(crate) fn execute_skill_install(...same parameters...) -> Outcome;
    pub(crate) fn execute_skill_update(...same parameters...) -> Outcome;
    pub(crate) fn execute_skill_remove(...same parameters...) -> Outcome;
}
```

The module also owns `NativeLifecycleSpec`, `SkillDestination`, Git skill
resolution, native lifecycle/skill operation IDs, state seed/projection
helpers, and daemon result normalization. `NativeLifecycleKind` and
`SkillInstallRequest` stay declared/re-exported by `application.rs` because
`entrypoint.rs` constructs them.

**Implementation Notes**:

- Preserve direct native argument construction and bounded process calls; do
  not extract or generalize the harness adapter here.
- Keep update policy and compatibility acknowledgment behavior unchanged,
  including generic `acknowledged: bool` propagation and hard drift blocks.
- Import document/scope/status helpers from the parent (`super::{...}`) rather
  than moving their behavior into lifecycle prematurely.
- Expose `lifecycle_operation_id` and `normalize_daemon_noop_result` as
  `pub(super)` and re-export them from the parent for the existing tests.
- Verify native lifecycle, skill, daemon, and Git-source tests after this step;
  repeat an unchanged operation where the existing suite already asserts
  idempotence.

**Acceptance Criteria**:

- [ ] All six application lifecycle entry points compile from the new module
  with unchanged parameter and return types.
- [ ] Native marketplace/plugin lifecycle, skill tree validation, Git SHA
  tracking, state journaling, and daemon records are behaviorally identical.
- [ ] Existing CLI output/error strings and operation IDs are unchanged.
- [ ] Workspace fmt, clippy, and tests pass.

**Risk**: This is the largest extraction and has many cross-helper references;
an accidental import replacement could alter which adapter or projection runs.
  
**Rollback**: Revert only the lifecycle extraction commit; the execution-port
  module and native behavior return to the pre-step layout without data cleanup.

---

### Step 3: Extract instruction status, setup, and bridge helpers

**Priority**: High  
**Risk**: Medium  
**Source Lens**: missing abstraction (instruction bridge policy mixed with lifecycle code)  
**Files**: `crates/cli/src/application.rs`, new `crates/cli/src/application/instructions.rs`, `crates/cli/src/application/execution.rs` (imports only)  
**Story**: `story-split-status-application-instructions`  
**Depends On**: `story-split-status-application-execution-ports`

**Current State**:

Instruction health/status, preview, setup/repair, duplicate nested Claude
bridge handling, canonical `AGENTS.md` creation, symlink/import publication,
backup paths, and bridge status helpers are interleaved with lifecycle and
reconciliation code (methods around lines 918–1,170, 3,031–3,780; helpers
4,404–4,585).

**Target State**:

```rust
// application/instructions.rs
impl StatusApplication<'_> {
    pub(crate) fn execute_instruction_status(&self, args: &ScopedOutputArgs) -> Outcome;
    pub(crate) fn execute_instruction_setup(...same parameters...) -> Outcome;
    pub(super) fn execute_instruction_setup_for_target(...same parameters...) -> Outcome;
    pub(super) fn execute_instruction_reconciliation_preview(...same parameters...) -> Outcome;
}
```

`instruction_locations`, `preferred_instruction_bridge_path`, resource/operation
ID helpers, `instruction_desired_resource`, bridge health/status functions, and
`instruction_backup_path` move with the methods. `InstructionPort` and
`InstructionWrite` remain in `execution.rs` and are imported privately.

**Implementation Notes**:

- Preserve global `~/AGENTS.md`, root project `AGENTS.md`, nested-only Claude
  bridge detection, duplicate consolidation/backup, symlink vs import mode,
  and `--yes` repair behavior exactly.
- Keep `ClaudeInstructionMode` and all filesystem operations at the same
  boundaries; do not make instruction code depend on concrete repositories.
- Keep preview and setup using identical path-selection helpers so plan output
  does not drift from sync behavior.

**Acceptance Criteria**:

- [ ] Global and project instruction status/setup/repair tests pass, including
  nested Claude duplicate and symlink/import cases.
- [ ] Plan's instruction preview and sync's instruction setup produce the same
  operation IDs, paths, warning codes, and result classes as before.
- [ ] No public CLI signature, output schema, or filesystem safety check changes.
- [ ] Workspace fmt, clippy, and tests pass.

**Risk**: Moving bridge helpers can accidentally change relative path context or
backup ordering, especially for nested project bridges.  
**Rollback**: Revert the instruction extraction commit; no production code
outside private module placement needs migration.

---

### Step 4: Extract plan/sync reconciliation orchestration

**Priority**: High  
**Risk**: High  
**Source Lens**: code smell / missing abstraction (candidate projection and
mutation aggregation span unrelated command flows)  
**Files**: `crates/cli/src/application.rs`, new `crates/cli/src/application/reconciliation.rs`, lifecycle/instructions child modules (imports only)  
**Story**: `story-split-status-application-reconciliation`  
**Depends On**: `story-split-status-application-lifecycle`, `story-split-status-application-instructions`

**Current State**:

`execute_plan`, `execute_sync`, and the private `execute_reconciliation` method
assemble selected inventory resources, project them to targets, route each
resource to lifecycle/instruction adapters, merge child outcomes, and classify
plan/sync results (lines 633–651 and 3,780–4,290). Selector matching and
resource source/name derivation are top-level helpers.

**Target State**:

```rust
// application/reconciliation.rs
impl StatusApplication<'_> {
    pub(crate) fn execute_plan(&self, args: &PlanArgs) -> Outcome;
    pub(crate) fn execute_sync(&self, args: &SyncArgs) -> Outcome;
    fn execute_reconciliation(
        &self, command: &'static str, target: &TargetArgs, scope: &ScopeArgs,
        includes: &[OperationSelector], excludes: &[OperationSelector], acknowledged: bool,
    ) -> Outcome;
}
```

The child module owns `merge_reconciliation_outcome`, selector matching,
resource source/name projection, scope argument conversion, and result merging
used only by reconciliation. It calls the existing lifecycle/instruction
methods through `StatusApplication` without introducing a second executor.

**Implementation Notes**:

- Preserve desired-inventory selection, target filtering, explicit include /
  exclude selectors, plan side-effect freedom, sync aggregation, and the
  documented generic `acknowledged` flag.
- Keep lifecycle adapters as the only mutation path; reconciliation remains an
  orchestrator and does not duplicate native process, lock, or journal code.
- Ensure daemon cycle remains in lifecycle (it calls skill/native adapters) and
  uses shared outcome/result helpers through `pub(super)` imports.

**Acceptance Criteria**:

- [ ] Populated `plan` and `sync` route every supported resource kind through
  the same child adapter and preserve operation counts/statuses.
- [ ] Empty inventory, selectors, all-scopes/project scope, target filtering,
  partial acknowledgment, and observation-failure results remain unchanged.
- [ ] `plan` remains mutation-free and repeated `sync` remains idempotent.
- [ ] Existing and release-gate reconciliation tests pass with no output diff.

**Risk**: A changed helper import or child outcome merge can alter result
classification while still compiling; this step needs focused CLI e2e review.
  
**Rollback**: Revert the reconciliation extraction commit and restore its
`impl`/helpers to `application.rs`; lifecycle and instruction modules remain
valid independently.

---

### Step 5: Extract status, adoption, and native observation projection

**Priority**: Medium  
**Risk**: Medium  
**Source Lens**: code smell (status/adoption projection is a separate read/adopt surface)  
**Files**: `crates/cli/src/application.rs`, new `crates/cli/src/application/status.rs`, `crates/cli/src/application/reconciliation.rs` (shared helper imports only)  
**Story**: `story-split-status-application-status-adoption`  
**Depends On**: `story-split-status-application-reconciliation`

**Current State**:

`execute`, `execute_adopt`, `StatusDocuments`, `StatusScope`, `StatusTargets`,
`StatusProjection`, `NativeObservation`, adoption projection/error mapping,
document loading, scope resolution, and observation/path helpers occupy the
final ~2,500 lines (roughly 4,290–6,823).

**Target State**:

```rust
// application/status.rs
impl StatusApplication<'_> {
    pub(crate) fn execute(&self, args: &StatusArgs) -> Outcome;
    pub(crate) fn execute_adopt(&self, args: &AdoptArgs) -> Outcome;
}

pub(super) struct StatusDocuments { /* same validated loaded documents */ }
pub(super) struct StatusScope { /* same resolved/output scope */ }
pub(super) struct StatusTargets { /* same enabled/selected harnesses */ }
pub(super) struct NativeObservation { /* same environment/resources/warnings */ }
```

Status/adoption helpers remain private to `status.rs`, while `load_documents`
and `scope_request` stay available to all child modules as `pub(super)` methods
or a narrowly re-exported support module. `entrypoint.rs` remains unchanged.

**Implementation Notes**:

- Preserve first-use detection, no-write status/adopt observation semantics,
  malformed-document redaction, target/scope errors, native observation
  normalization, adoption revalidation, and health/output ordering.
- Keep filesystem and harness observation adapters injected through the same
  `NativeObservationMode`; do not add a status cache or persist snapshots.
- Keep `application/tests.rs` in place and validate all existing status and
  adoption assertions after the move.

**Acceptance Criteria**:

- [ ] Status and adoption produce byte/schema-compatible output and identical
  exit classifications for first use, missing documents, invalid targets,
  global/project/all-scope, native failures, and conflicts.
- [ ] Adoption remains the only path that mutates inventory from native
  observation; status remains read-only.
- [ ] Workspace fmt, clippy, unit tests, compiled-binary tests, and isolated
  e2e tests pass.

**Risk**: Observation helper visibility and ordering are easy to disturb; a
status refactor that changes when a warning is appended is behavior-changing.
  
**Rollback**: Revert the status extraction commit. The prior reconciliation and
lifecycle boundaries are independent and can remain until the extraction is
re-attempted.

## Cross-step Acceptance and Verification

- [ ] `entrypoint.rs` still constructs the same `StatusApplication` fields and
  invokes the same `pub(crate)` methods; no CLI composition diff is required.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo test --workspace --all-targets --offline` passes.
- [ ] `cargo clippy --workspace --all-targets --offline -- -D warnings` passes.
- [ ] `git diff --check` passes and each step is committed separately.
- [ ] Existing command fixtures are run in isolated test-support roots; no
  developer HOME/XDG/native harness state is touched.
- [ ] A final fresh-context review confirms no public signature, output schema,
  error code/message, operation ID, lock/journal, or filesystem ordering drift.

## Implementation Order

1. `story-split-status-application-execution-ports`
2. `story-split-status-application-lifecycle` (after step 1)
3. `story-split-status-application-instructions` (after step 1; parallel with step 2 is safe)
4. `story-split-status-application-reconciliation` (after steps 2 and 3)
5. `story-split-status-application-status-adoption` (after step 4)

The feature remains pure refactoring. If implementation uncovers a behavior
change or bug (including a missing `plan`/`sync` candidate), stop the extraction
and route that change as a separate non-`[refactor]` item rather than hiding it
in a module move.

## Children complete (2026-07-12)

All five extraction children reached `stage: done` after review. Their
implementation commits are `0a4814d`, `cc3a5a6`, `411904f`, `e3e1192`, and
`de3c132`; the supporting refactors are `209564f`, `53f2f81`, and `d78d3af`.
The bundle preserves public signatures, command output/error contracts,
operation IDs, lifecycle ordering, and filesystem/state boundaries. Shared
private helper support intentionally remains in `application.rs` where it is
used across sibling modules.

## Review (2026-07-12)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: The implementation keeps several shared helper types and projections
in `application.rs` instead of relocating every helper into its responsibility
module as the step sketches suggest. The resulting private module graph is
coherent and the choice preserves shared support without changing behavior.

**Notes**: Deep feature review in a fresh same-harness context at standard
weight. Aggregate review covered design alignment, public and CLI contracts,
serialization/output and error stability, operation IDs, state/journal and
filesystem ordering, native lifecycle routing, plan mutation-freedom, sync
selection, status read-only behavior, adoption locking, and foundation-doc
alignment. `cargo fmt --all -- --check`, `cargo test --workspace --all-targets
--offline`, `cargo clippy --workspace --all-targets --offline -- -D warnings`,
and `git diff --check` all pass.
