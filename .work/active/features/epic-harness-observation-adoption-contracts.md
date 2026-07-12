---
id: epic-harness-observation-adoption-contracts
kind: feature
stage: done
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: []
release_binding: 3.0.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Normalized Observation Contracts

Define the pure contracts every adapter and consumer shares: scope-bearing
resource instances and exact selectors; stable lineage independent of mutable
revision; typed harness installation and executable identity; compiled profile
selection and scope-varying capabilities; one-concrete-scope observation
requests; declared/effective snapshots; source-bearing observed resources; safe
open finding codes/fields; adapter and coordinator ports; and ephemeral
observation semantics. Update strict storage/domain schemas and foundation
wording as required by the clean-break v3 contract. No native format parser,
process execution, filesystem traversal, status rendering, or persistence
workflow belongs here.

## Design

### Resource instance identity

`ResourceId` remains the stable logical/user selector and receives its own
validated alphabet so documented qualified plugin IDs can contain `@` without
relaxing `HarnessId`, `OperationId`, or `ComponentId`. New `ResourceKey { id,
scope }` is the single source of truth for a concrete instance. Desired,
observed, state, dependency, operation-selector, and managed-owner contracts use
the exact key. Cross-scope dependencies are legal only when they name an exact
key. Mutable revision/version/fingerprint never enters either identity.

Wire shapes use one nested `key` rather than sibling `id` and `scope`, making a
mismatch unrepresentable. `ObservationKey` is `{ resource: ResourceKey,
harness, layer }`. `ObservedResource` derives its scope from that key. Managed
artifact path derivation hashes the canonical scope plus logical ID so equal IDs
in global and multiple projects cannot alias. Because v3 is unreleased and
explicitly clean-break, strict schema-1 fixtures change in place and prior
ResourceId-only shapes are rejected without migration code.

### Observation and finding contracts

Fresh normalized observations are ephemeral values. `ObservedResource` carries
a typed optional `Source` and no arbitrary JSON metadata. Observed dependencies
retain exact keys and resolved/unresolved evidence so one malformed native edge
does not invalidate healthy siblings. Findings use source-registered codes,
authored static summaries, severity, harness/scope/resource subjects, and a
small registered typed scalar field vocabulary; constructors cannot accept raw
native output, settings objects, arbitrary JSON, or dynamic messages. Runtime-
open diagnostic codes were rejected during review because identifier-shaped
secrets could otherwise enter rendered and serialized output.

The installation/profile vocabulary separates configured binary, resolved
absolute executable identity, opaque native version, compiled profile ID,
profile authority, reachability, and scoped capability sets. A profile may
narrow support per global/project scope. Unknown versions have no verified
profile and expose no mutation capability set.

`ObservationRequest` always contains one concrete scope plus installation and
profile evidence bound to the same executable. `HarnessObservation` returns
normalized resources/findings for one harness/scope. `ObservedEnvironment`
aggregates requested scopes and per-harness results without hiding failures.
Core defines behavior ports for adapters and the observation coordinator;
concrete registries, native formats, I/O, timeouts, persistence, and CLI output
remain downstream.

### Pre-mortem

- **Only maps become scope-aware.** Dependencies, selectors, state, and managed
  ownership migrate in separate compiling steps before any snapshot type lands.
- **Old wire shapes accidentally deserialize.** Golden and adversarial tests
  require the previous sibling `id`/`scope` and ResourceId-only owner shapes to
  fail strictly.
- **Scope changes public artifact paths.** Same-ID different-scope fixtures
  prove distinct deterministic paths; same key/fingerprint remains byte-stable.
- **Finding APIs become a secret channel.** No dynamic string/JSON constructor
  exists; canary values cannot appear in Debug, Display, serde, or generated
  output fixtures.
- **Adapter ports smuggle native DTOs into core.** Only normalized domain types
  cross the port; opaque raw payloads remain private to harness implementations.

## Implementation units

1. `epic-harness-observation-adoption-contracts-resource-key` — add
   `ResourceKey`, ResourceId-specific qualified spelling, canonical key
   encoding, and identity/serde/order/hash contracts — depends on `[]`.
2. `epic-harness-observation-adoption-contracts-resource-graph` — migrate
   desired/observed keys, exact cross-scope dependencies, graph errors/maps,
   source-bearing observations, and resolved/unresolved native edges — depends
   on `[epic-harness-observation-adoption-contracts-resource-key]`.
3. `epic-harness-observation-adoption-contracts-operation-selectors` — migrate
   resource/component selectors and acknowledgment coverage to exact keys and
   enforce semantic-scope coherence — depends on
   `[epic-harness-observation-adoption-contracts-resource-graph]`.
4. `epic-harness-observation-adoption-contracts-managed-ownership` — migrate
   artifact records, repository ports, errors, residuals, and path hashing to
   exact scope-bearing owners — depends on
   `[epic-harness-observation-adoption-contracts-resource-key]`.
5. `epic-harness-observation-adoption-contracts-storage-wires` — migrate strict
   inventory/state maps, errors, goldens, repository and cross-layer tests;
   reject old shapes with no migration — depends on
   `[epic-harness-observation-adoption-contracts-resource-graph,
   epic-harness-observation-adoption-contracts-managed-ownership]`.
6. `epic-harness-observation-adoption-contracts-findings` — replace arbitrary
   observation messages/metadata with safe code/summary/severity/subject/typed
   fields and secret-canary contracts — depends on
   `[epic-harness-observation-adoption-contracts-resource-graph]`.
7. `epic-harness-observation-adoption-contracts-installation-profiles` — add
   executable, installation, opaque version, profile authority/identity, and
   scope-aware capability values — depends on
   `[epic-harness-observation-adoption-contracts-resource-key]`.
8. `epic-harness-observation-adoption-contracts-snapshots-ports` — add concrete
   scope requests, harness/environment snapshots, safe errors, and adapter/
   coordinator behavior ports — depends on
   `[epic-harness-observation-adoption-contracts-storage-wires,
   epic-harness-observation-adoption-contracts-findings,
   epic-harness-observation-adoption-contracts-installation-profiles]`.
9. `epic-harness-observation-adoption-contracts-foundation-integration` — align
   capability authority, identity/version, missing-config, ephemeral-state,
   and shared-Claude wording; verify strict cross-layer serialization and
   compiled CLI compatibility — depends on
   `[epic-harness-observation-adoption-contracts-operation-selectors,
   epic-harness-observation-adoption-contracts-snapshots-ports]`.

## Implementation

- All nine child stories are done and independently reviewed.
- Exact scope-bearing keys now span resources, dependencies, inventory/state,
  managed ownership, artifact paths, operation selectors, acknowledgments, and
  snapshot contexts without mutable revision/version identity.
- Installation/profile evidence binds one reachable executable and version;
  verified compiled profiles alone grant mutation authority and probes only
  narrow support. Unknown versions remain observable and expose no mutation
  capabilities.
- Findings and adapter errors have safe typed vocabularies; observation
  requests and complete partial environments are ephemeral normalized values
  behind core behavior ports.
- Foundation, README, website, generated LLM documents, first-use policy, and
  cross-layer tests now match the compiled contracts.

## Verification

- Locked formatting, workspace/all-target checks and tests, warnings-denied
  Clippy/rustdoc, release build, and compiled-binary verification pass.
- The workspace has 226 passing Rust tests, including 3 cross-layer foundation
  tests and 6 compiled-binary tests.
- Website build and repeated byte-identical LLM documentation generation pass.

## Review

- Approved after a same-harness fresh-context review across all nine child
  stories and their cross-layer seams; no actionable findings remained.
- Confirmed exact-key identity, evidence-only revisions, strict legacy-wire
  rejection, compiled-only mutation authority, safe typed diagnostics,
  complete ephemeral snapshots, and disabled first-use defaults.
- Clean-tree verification passed all 226 Rust tests, warnings-denied Clippy and
  rustdoc, release and compiled-binary checks, website build, and deterministic
  generated documentation.

## Acceptance criteria

- Equal logical IDs in global and multiple project scopes coexist in every
  graph/document and remain distinct in Eq/Ord/Hash/serde and managed paths.
- Exact cross-scope dependencies resolve; dangling/self/cycle diagnostics carry
  exact keys. Unresolved native edges remain observable rather than aborting a
  snapshot.
- Operation selectors cannot contradict operation semantic scope; consequence
  and component coverage remains exact.
- Strict inventory/state/artifact wires use nested keys, reject prior shapes
  and unknown fields, and remain deterministic/idempotent at schema 1.
- Findings and errors cannot serialize/render/persist raw native bytes,
  settings, argv, arbitrary JSON, or dynamic messages.
- Profiles vary by scope, compiled authority is explicit, and unknown versions
  cannot represent verified mutation support.
- Adapter/coordinator ports expose normalized ephemeral data only and core
  remains independent of concrete harnesses and CLI.
- Full locked format/check/Clippy/test/rustdoc and optimized compiled-binary
  ladders pass.
