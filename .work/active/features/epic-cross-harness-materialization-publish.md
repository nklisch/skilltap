---
id: epic-cross-harness-materialization-publish
kind: feature
stage: done
tags: []
parent: epic-cross-harness-materialization
depends_on: [epic-cross-harness-materialization-skills-mcp, epic-cross-harness-materialization-hooks]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Managed Target Artifacts

Publish target artifacts atomically, preserve source provenance and ownership,
and verify that each target harness loads the projected resource.

## Architectural choice

Use a three-boundary publication pipeline: core first validates and orders a
pure publication batch, the managed-artifact repository publishes complete
trees atomically, and a harness adapter performs a fresh effective-load
verification before state is refreshed. A single CLI-side copy routine would
couple planning to filesystem details and could record ownership before the
target actually loads the resource. Native registration remains an explicit
operation when the adapter supports it; file projection is only the fallback
for a faithful or explicitly acknowledged partial plan.

## Design decisions

- **When is ownership recorded?** Only after every requested target projection
  has either published successfully and passed load verification, or the
  operation has a typed partial result whose exact selectors were acknowledged.
  A failed target never becomes owned merely because its managed tree exists.
- **How are multiple target trees committed?** Publish in deterministic target
  order under the existing configuration lock, then publish one state update
  containing all successful artifact records. A later-target failure leaves the
  earlier artifact as an explicitly recoverable partial publication; it is not
  hidden by a best-effort rollback.
- **What counts as verification?** The adapter must re-observe the target's
  documented effective load path and prove the projected resource identity and
  fingerprint are present. Cache inspection is never verification.

## Implementation Units

### Unit 1: Pure publication batch (trickiest unit)
**File**: `crates/core/src/publication.rs`
**Story**: `epic-cross-harness-materialization-publish-batch`

```rust
pub struct PublicationEntry {
    pub resource: ResourceKey,
    pub target: HarnessId,
    pub role: ArtifactRole,
    pub fingerprint: Fingerprint,
    pub tree: ArtifactTree,
}

pub struct PublicationBatch {
    entries: Vec<PublicationEntry>,
}

pub fn plan_publication(
    entries: impl IntoIterator<Item = PublicationEntry>,
) -> Result<PublicationBatch, PublicationPlanError>;
```

**Implementation Notes**:
- Reject duplicate resource/target pairs, empty trees, target-inconsistent
  fingerprints, and non-deterministic ordering before any repository call.
- Keep complete directory trees intact, including the required top-level
  `SKILL.md`; do not reduce a skill to a single file.
- Publication planning has no I/O and no state mutation.

**Acceptance Criteria**:
- [x] Batch entries sort deterministically by scope-bearing resource and target.
- [x] Duplicate or malformed entries fail before publication.
- [x] Replanning the same entries produces byte- and order-stable output.

### Unit 2: Managed artifact transaction
**File**: `crates/core/src/publication.rs` and `crates/cli/src/application.rs`
**Story**: `epic-cross-harness-materialization-publish-transaction`

```rust
pub trait PublicationSink {
    type Error;

    fn publish(
        &self,
        entry: &PublicationEntry,
    ) -> Result<PublishedArtifact, Self::Error>;
}

pub fn apply_publication(
    batch: &PublicationBatch,
    sink: &impl PublicationSink,
) -> Result<PublicationReceipt, PublicationApplyError>;
```

**Implementation Notes**:
- Adapt `ManagedArtifactRepository` rather than creating a second tree writer.
- Hold the existing configuration lock across publication and state commit;
  preserve typed residuals if a later entry fails.
- Native lifecycle registration precedes file materialization when available;
  a native failure prevents a fallback write unless the plan explicitly chose
  managed projection.

- **Acceptance Criteria**:
- [x] A repeated apply is a no-op when the complete tree and fingerprint match.
- [x] A conflict or partial publication is surfaced with target and resource
  identity and never silently marked healthy.
- [x] No unmanaged destination is removed or overwritten.

### Unit 3: Effective-load verification and state receipt
**File**: `crates/core/src/publication.rs`, `crates/harnesses/src/lib.rs`, and `crates/cli/src/application.rs`
**Story**: `epic-cross-harness-materialization-publish-verification`

```rust
pub trait LoadVerifier {
    fn verify_loaded(
        &self,
        entry: &PublicationEntry,
        artifact: &PublishedArtifact,
    ) -> Result<VerifiedTarget, LoadVerificationError>;
}

pub struct PublicationReceipt {
    pub published: Vec<PublishedArtifact>,
    pub verified: Vec<VerifiedTarget>,
}
```

**Implementation Notes**:
- Verification consumes fresh bounded observations from Codex/Claude adapters
  and compares normalized identity/fingerprint, not cache records.
- State refresh uses the existing `ResourceState::new` and
  `StateDocument::refresh_resource_state` invariants, preserving prior apply
  history and source provenance.
- A verification mismatch is a typed attention result; it cannot be converted
  into success by a generic confirmation flag.

- **Acceptance Criteria**:
- [x] Successful publication records managed ownership only after effective
      load verification.
- [x] Verification failures retain enough typed context for `status` without
      retaining native secrets or raw payloads.
- [x] State publication is atomic across the successfully verified entries.

## Implementation Order

1. `epic-cross-harness-materialization-publish-batch`
2. `epic-cross-harness-materialization-publish-transaction`
3. `epic-cross-harness-materialization-publish-verification`

## Testing

- Pure batch tests cover ordering, duplicate targets, empty trees, and required
  `SKILL.md` complete-tree shape.
- Transaction tests use the existing no-follow filesystem fixtures to prove
  idempotence, conflict handling, lock scope, and residual reporting.
- Adapter tests use fresh effective observations and prove caches are ignored;
  integration tests prove state is not owned before verification.

## Implementation Notes

- All three child stories are complete. The core publication boundary now
  validates deterministic batches, delegates complete-tree writes to the
  existing managed-artifact repository, verifies fresh effective observations,
  and records ownership/native identities in one pure state refresh.
- Harnesses expose `EffectiveObservationVerifier`; cache reads are not part of
  the verification path.
- Targeted core/harness tests and clippy pass. Full workspace verification is
  the remaining release gate.

## Review Record

- Inline deep review: **pass**. The implementation
  preserves the lock/state handoff contract and fails closed on absent,
  unhealthy, or mismatched effective observations.

## Risks

The riskiest assumption is that every native harness can expose a bounded,
fresh effective observation after a managed projection. If a documented load
path cannot be observed, the target remains pending/blocked and no ownership
record is written; skilltap does not infer success from files on disk.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this
  autopilot run is intentionally single-agent.
