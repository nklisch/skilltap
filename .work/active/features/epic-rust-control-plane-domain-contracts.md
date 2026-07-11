---
id: epic-rust-control-plane-domain-contracts
kind: feature
stage: implementing
tags: []
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-workspace-reset]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Control-Plane Domain Contracts

## Brief

Define the validated `skilltap-core` vocabulary shared by every later
capability: harness and resource identities, scope and target selection, source
and revision identities, desired and observed resource graphs, provenance,
capabilities, compatibility, fingerprints, operation results, and attention
reasons. Boundary constructors reject invalid raw values so internal code does
not exchange unvalidated strings for identity-bearing concepts.

This feature establishes types and invariants, not reconciliation algorithms,
harness-specific interpretation, persistence implementations, or CLI rendering.
Harness-specific metadata remains namespaced and opaque to the general domain.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: shared-contract producer — storage and runtime primitives
  depend on its validated types

## Foundation references

- `docs/ARCH.md` — Domain Model, Core Types, Dependency Direction
- `docs/SPEC.md` — Terminology, Operating Model, Skill Compatibility
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Identity Mapping

## Design decisions

- Use small public modules under `skilltap_core::domain` rather than one domain
  file. Re-export the stable vocabulary from `domain` and keep validation
  helpers private.
- Add `serde` as a production dependency and `serde_json` for opaque adapter
  metadata and boundary tests. Every identity-bearing type validates during
  construction and deserialization; callers cannot bypass invariants with raw
  strings.
- Keep concrete scope separate from command selection: `Scope` is only global
  or one canonical project path, while `ScopeSelection` may also be
  `AllScopes`. Likewise, `TargetSelection` resolves to a non-empty deterministic
  `HarnessSet` before adapters receive work.
- Model skill behavioral compatibility (`compatible`, `target-specific`,
  `unknown`, `incompatible`) separately from cross-harness transfer fidelity
  (`faithful`, `materializable`, `partial`, `blocked`). Collapsing them would
  confuse a valid target-specific native skill with a faithful transfer.
- Use deterministic `BTreeMap`/`BTreeSet` collections. Harness-specific metadata
  is namespaced by `HarnessId` and stored as opaque serialized values; core
  contracts carry but never interpret it.
- A `Plan` constructor enforces unique operation ids, known dependencies, and
  an acyclic graph. Acknowledgment-required operations enumerate exact resource
  or component selectors and material consequences; no boolean field can erase
  incompatibility evidence.

## Public contract shape

```text
skilltap_core::domain
├── identity      HarnessId, ResourceId, OperationId, NativeId
├── scope         AbsolutePath, RelativeArtifactPath, Scope,
│                 ScopeSelection, HarnessSet, TargetSelection
├── source        Source, SourceKind, SourceLocator, RequestedRevision,
│                 ResolvedRevision, GitCommit, Fingerprint
├── resource      ResourceKind, ComponentKind, Provenance, Ownership,
│                 DesiredResource, ObservedResource, ObservationFinding,
│                 ResourceGraph
├── capability    CapabilityId, CapabilitySupport, CapabilitySet
├── compatibility CompatibilityClass, TransferFidelity,
│                 CompatibilityEvidence, CompatibilityResult
└── operation     Plan, Operation, OperationDependency, OperationClass,
                  Reversibility, AcknowledgmentRequirement, ApplyResult,
                  OperationResult, AttentionReason
```

Validated text newtypes reject empty or surrounding-whitespace values, control
characters, and excessive length. Paths must be UTF-8 and lexically absolute
or relative as declared; relative artifact paths reject `..`. Git commits accept
40- or 64-character hexadecimal object ids. Fingerprints carry an explicit
algorithm and validated digest rather than an unstructured string. Runtime
filesystem canonicalization and Git resolution remain outside this feature.

`DesiredResource` and `ObservedResource` share stable identity, kind, concrete
scope, and dependency vocabulary but remain separate types. Desired resources
carry requested targets/source/update intent. Observed resources carry native
identities, revisions, fingerprints, provenance/ownership, health, and opaque
per-harness metadata. Malformed unmanaged native entries are represented as
`ObservationFinding` values rather than forced into a valid resource.

## Implementation units

### Unit 1: Validated identities, scope, and source primitives

**Story:** `epic-rust-control-plane-domain-contracts-identity-scope-source`

**Files:** workspace/core manifests; `crates/core/src/domain/mod.rs`,
`identity.rs`, `scope.rs`, `source.rs`, and focused unit tests.

Implement the scalar boundary vocabulary, serde round trips, deterministic
target resolution, path validation, revision forms, and fingerprint parsing.
Validation errors are typed, structured, and free of terminal rendering.

### Unit 2: Desired and observed resource graphs

**Story:** `epic-rust-control-plane-domain-contracts-resource-graph`

**Files:** `crates/core/src/domain/resource.rs` and tests.

Implement resource/component kinds, provenance and ownership, native identity
mapping, opaque adapter metadata, desired/observed resource records, observation
findings, and a graph constructor that rejects duplicate ids, dangling
dependencies, self-dependencies, and cycles.

### Unit 3: Capability and compatibility evidence

**Story:** `epic-rust-control-plane-domain-contracts-capability-compatibility`

**Files:** `crates/core/src/domain/capability.rs`, `compatibility.rs`, and tests.

Implement extensible dotted capability identifiers and their
supported/unsupported/unverified status. Implement the two-axis compatibility
model with target, affected components, machine-readable evidence codes, and
human-readable consequences. Non-faithful results require evidence.

### Unit 4: Plans, operation dependencies, and results

**Story:** `epic-rust-control-plane-domain-contracts-plan-results`

**Files:** `crates/core/src/domain/operation.rs` and tests.

Implement the serializable operation graph, classifications, reversibility,
piecewise acknowledgment requirements, structured attention reasons, and
per-operation/final apply outcomes. Constructors reject unknown dependencies,
cycles, empty consequence sets, and internally inconsistent result summaries.
This is contract validation only; no planner or executor algorithm is added.

## Implementation order

1. Identity, scope, and source primitives.
2. Resource graph and capability/compatibility units in parallel.
3. Plan and result contracts after both prior units.

## Testing

- Unit-test every accepted and rejected constructor boundary, including serde
  deserialization so invalid persisted strings cannot bypass constructors.
- Assert stable snake_case serialized forms for public enums and tagged unions.
- Property-style table tests cover path traversal, control characters, invalid
  hex, empty target sets, duplicate graph ids, dangling edges, self edges, and
  multi-node cycles.
- Round-trip representative desired/observed graphs, compatibility evidence,
  partial-operation acknowledgments, and partial apply results through JSON.
- Run format, locked clippy with warnings denied, and the complete workspace
  test suite after every story.

## Risks

- Over-constraining native identifiers would make adapters lossy. Native ids and
  version strings therefore use bounded opaque validation; only skilltap-owned
  ids, paths, hashes, and capability keys use stricter grammars.
- Mixing compatibility axes would make later safety decisions ambiguous. The
  separate behavioral and transfer types are mandatory even when a caller
  commonly reports them together.
- Premature storage-document types would couple this feature to TOML/JSON file
  versions. This feature supplies reusable serializable values; the storage
  feature owns document schemas and migration policy.
