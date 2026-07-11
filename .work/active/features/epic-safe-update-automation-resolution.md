---
id: epic-safe-update-automation-resolution
kind: feature
stage: done
tags: []
parent: epic-safe-update-automation
depends_on: [epic-native-marketplace-plugin-lifecycle, epic-standalone-skill-lifecycle]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve Concrete Updates

Resolve explicit Git refs and native versions to concrete SHA/revision
candidates without mutating resources.

## Design decisions

- **What is a concrete candidate?** A typed `ResolvedRevision` (Git commit or
  native revision), never a display version or raw command output.
- **How are tracking Git sources checked?** Resolve the requested ref or the
  remote default through a bounded direct Git query. Do not checkout, rewrite
  managed skills, or mutate a harness during resolution.
- **How are native candidates resolved?** Reuse the fresh harness observation
  pipeline and verified capability profiles. Unknown or unverified native
  versions remain unresolved and are reported as attention, never guessed.
- **What may be persisted?** A check may atomically cache the available
  revision and check timestamp in `state.json`; it never changes desired
  inventory, installed artifacts, or native configuration.

## Architectural choice

Use a pure core candidate builder with injected revision-resolution ports and
keep Git/native adapters in `skilltap-harnesses`. A CLI-owned resolver would
duplicate update policy and make daemon behavior diverge; a resolver that edits
the managed checkout or invokes native update commands would make a read/check
operation destructive. Typed revisions and bounded error categories make the
result safe for both foreground status and the future daemon.

## Implementation Units

### Unit 1: Typed resolution contract and candidate construction (trickiest unit)
**File**: `crates/core/src/updates.rs`
**Story**: `epic-safe-update-automation-resolution-contract`

```rust
use crate::domain::{DesiredResource, HarnessId, ResolvedRevision, Source};

pub struct UpdateResolutionRequest<'a> {
    pub resource: &'a DesiredResource,
    pub installed: Option<&'a ResolvedRevision>,
}

pub enum ResolutionError {
    UnreachableSource,
    InvalidRequestedRevision,
    UnsupportedSourceKind,
    NativeObservationUnavailable,
}

pub trait SourceRevisionResolver {
    fn resolve(&self, source: &Source) -> Result<ResolvedRevision, ResolutionError>;
}

pub trait NativeRevisionResolver {
    fn resolve(
        &self,
        resource: &DesiredResource,
        target: &HarnessId,
    ) -> Result<Option<ResolvedRevision>, ResolutionError>;
}

pub struct ResolvedUpdate {
    pub current: Option<ResolvedRevision>,
    pub available: Option<ResolvedRevision>,
    pub error: Option<ResolutionError>,
}

pub fn resolve_candidate<R: SourceRevisionResolver, N: NativeRevisionResolver>(
    source_resolver: &R,
    native_resolver: &N,
    request: UpdateResolutionRequest<'_>,
) -> ResolvedUpdate;
```

**Implementation Notes**:
- Replace string revisions in `UpdateCandidate` with typed revisions and
  classify equality before any safety flags are considered.
- A source resource resolves once; native resources resolve per selected target
  and merge only when all targeted observations agree. Disagreement becomes an
  explicit unresolved result rather than an arbitrary winner.
- Keep `ResolutionError` bounded and serializable by the CLI layer without raw
  argv, stdout, stderr, or secrets.

**Acceptance Criteria**:
- [ ] Equal typed revisions produce `NoUpdate`; a changed revision produces a
      candidate without changing desired state.
- [ ] Pinned, drifted, compatibility-changed, and acknowledgment-required
      candidates retain the existing safety classifications.
- [ ] Any resolver error is visible and cannot be coerced into a safe update.
- [ ] Different native target revisions never collapse into one candidate.

### Unit 2: Git and native resolution adapters
**File**: `crates/harnesses/src/update_resolution.rs`
**Story**: `epic-safe-update-automation-resolution-adapters`

```rust
pub struct GitSourceRevisionResolver {
    process: Box<dyn NativeProcessRunner>,
}

impl SourceRevisionResolver for GitSourceRevisionResolver {
    fn resolve(&self, source: &Source) -> Result<ResolvedRevision, ResolutionError>;
}

pub struct ObservedNativeRevisionResolver<'a> {
    environment: &'a ObservedEnvironment,
}

impl NativeRevisionResolver for ObservedNativeRevisionResolver<'_> {
    fn resolve(
        &self,
        resource: &DesiredResource,
        target: &HarnessId,
    ) -> Result<Option<ResolvedRevision>, ResolutionError>;
}
```

**Implementation Notes**:
- Git resolution uses direct bounded `git ls-remote` arguments for the
  explicit locator and requested revision (or `HEAD`), validates the returned
  SHA with `GitCommit`, and never writes the source tree or managed skill
  destination. Credentials remain in Git's own environment.
- Local and remote-catalog sources return the typed unsupported error until a
  later feature supplies a documented resolver.
- Native resolution reads only fresh adapter observations and verified profile
  evidence. An unknown harness version or missing revision is unresolved.

**Acceptance Criteria**:
- [ ] Tracking and pinned Git refs resolve to the expected SHA in fixtures.
- [ ] Malformed, unreachable, or ambiguous Git output is rejected without a
      false candidate.
- [ ] Native observations return concrete revisions when present and typed
      attention when profile evidence is unavailable.
- [ ] No adapter test invokes install, update, checkout, or cache writes.

### Unit 3: Check orchestration and state cache
**File**: `crates/cli/src/application.rs` and `crates/core/src/storage/state.rs`
**Story**: `epic-safe-update-automation-resolution-orchestration`

```rust
pub fn check_updates(
    requests: &[UpdateResolutionRequest<'_>],
    source_resolver: &impl SourceRevisionResolver,
    native_resolver: &impl NativeRevisionResolver,
) -> Vec<ResolvedUpdate>;

impl StateDocument {
    pub fn with_available_revision(
        &self,
        resource: &ResourceKey,
        available: Option<ResolvedRevision>,
        checked_at: Timestamp,
    ) -> Result<Self, SchemaError>;
}
```

**Implementation Notes**:
- The application observes first, resolves explicit desired resources, renders
  candidates/warnings, and only then optionally publishes available revisions
  and `last_update_check` atomically. It never calls a mutating lifecycle path.
- State cache writes preserve native IDs, fingerprints, installed revisions,
  and operation journals. A failed check leaves state unchanged.
- JSON output contains candidate identity, current/available typed revisions,
  safety, and a bounded next action; human output remains concise.

**Acceptance Criteria**:
- [ ] `status`/future `update check` can repeat unchanged checks idempotently.
- [ ] Resolver failures do not write inventory, managed artifacts, native files,
      or partial state.
- [ ] Successful cache writes preserve existing operation history and are
      atomic across all checked resources.
- [ ] A candidate with a changed Git SHA is visible before any update command.

## Implementation Order

1. `epic-safe-update-automation-resolution-contract`
2. `epic-safe-update-automation-resolution-adapters`
3. `epic-safe-update-automation-resolution-orchestration`

## Testing

- Core unit tests cover typed revision equality, target disagreement, bounded
  resolution errors, and safety classification.
- Harness integration fixtures exercise Git refs, malformed output, unknown
  versions, and fresh native observation without mutation.
- CLI/state tests repeat checks, assert atomic cache behavior, and verify that
  desired inventory and native files are byte-for-byte unchanged.

## Risks

Remote Git servers can return multiple matching refs or omit `HEAD`. The
resolver must treat ambiguity or absence as unresolved and leave the update
decision to the user; it must never select a convenient ref silently. Native
version metadata can also be absent on older harnesses, in which case status
reports attention instead of inferring a version from cache paths.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this autopilot
  run is intentionally single-agent and no different model was selected.

## Implementation notes

- Completed child stories: `epic-safe-update-automation-resolution-contract`,
  `epic-safe-update-automation-resolution-adapters`, and
  `epic-safe-update-automation-resolution-orchestration`.
- Delivered typed revision candidates, bounded Git/native resolvers,
  read-only status update projection, and atomic available-revision state cache
  primitives.
- Verification: targeted core, harness, and CLI tests plus clippy passed; the
  full workspace suite remains the final feature review gate.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: status does not persist cache data, by design; the pure state cache
primitive is ready for the downstream foreground/daemon writer.

**Notes**: Deep feature review completed inline in degraded fresh-context mode
because this run is intentionally single-agent. The completeness pass verified
typed revisions, native target agreement, state journal preservation, and
read-only status projection. The adversarial pass added blocked classification
for unresolved candidates and confirmed bounded Git invocation and no native
mutation. Full workspace tests and clippy passed.
