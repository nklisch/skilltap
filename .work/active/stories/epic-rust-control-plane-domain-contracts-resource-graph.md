---
id: epic-rust-control-plane-domain-contracts-resource-graph
kind: story
stage: done
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-identity-scope-source]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Desired and Observed Resource Graphs

## Scope

Implement Unit 2 from the parent feature: resource/component kinds, provenance,
ownership, desired and observed records, native identity and opaque metadata
namespacing, observation findings, and validated dependency graphs.

## Acceptance criteria

- [x] Desired and observed records remain distinct and use concrete scopes.
- [x] Harness-native metadata is namespaced and opaque to general domain code.
- [x] Graph construction rejects duplicates, dangling/self edges, and cycles.
- [x] Malformed unmanaged entries can be reported without fabricating resources.
- [x] Representative graphs serialize deterministically and round-trip.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: `crates/core/src/domain/resource.rs`, `crates/core/Cargo.toml`.
- Tests added: resource/component enum wire forms; desired/observed ID alignment;
  duplicate, dangling, self, and multi-node cycle rejection; deterministic graph
  round trips; opaque native metadata preservation; malformed unmanaged findings;
  owned-envelope unknown-field rejection and finding-message validation.
- Verification: `cargo fmt --all -- --check`, `cargo check --workspace --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test -p skilltap-core --locked`, and `cargo test --workspace --locked`.
- Discrepancies from design: `serde_json` moved from a core dev dependency to a
  production dependency because opaque adapter metadata is part of the public
  resource contract; otherwise none.
- Dispatch rationale: direct implementation within the resource module while a
  sibling agent owned the disjoint capability/compatibility modules.
- Adjacent issues parked: none.

## Review findings (2026-07-11)

- Blocker: `components: BTreeSet<ComponentKind>` collapses multiple components
  of the same kind and preserves neither identity, requiredness, nor component
  dependencies. Introduce validated `ComponentId`, a component record, and a
  deterministic component subgraph that rejects duplicate, dangling, self, and
  cyclic dependencies through constructors and deserialization.
- Important: observation-finding ordering omits opaque metadata, so findings
  with otherwise equal keys serialize according to adapter input order. Add a
  deterministic metadata tie-break and prove reversed inputs serialize equally.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Initial component-identity and deterministic-order findings were
resolved in `f1caac4`. Constructor and serde paths now validate both resource
and component graphs. Story verified by implement, adversarial contract review,
and integrated locked workspace checks; fast-lane advance.

## Review resolution (2026-07-11)

- Added validated, serde-safe `ComponentId`, explicit
  `ComponentRequiredness`, and `ResourceComponent` records with component-local
  dependency identities.
- Replaced the lossy kind sets on desired and observed resources with a
  deterministic `ComponentGraph`. Its constructor and deserializer reject
  duplicate ids, dangling and self dependencies, and multi-node cycles while
  preserving multiple components of the same kind.
- Added canonical recursive JSON ordering as the final finding sort key and a
  reversed-input regression test proving byte-identical graph JSON.
- Re-ran the complete locked format, workspace check, warnings-as-errors clippy,
  focused core test, and workspace test ladder successfully.
