---
id: feature-managed-fallback-target-parity-contract
kind: story
stage: review
tags: []
parent: feature-managed-fallback-target-parity
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Managed Projection Port Contract and Pure Types

## Scope

Implement Unit 1 of the managed-fallback-target-parity feature design: the
`ManagedProjectionPort` adapter trait (in `skilltap-harnesses`) and its pure
supporting types (in `skilltap-core`), plus the defaulted
`HarnessAdapter::managed_projection() -> Option<&'static dyn
ManagedProjectionPort>` accessor. This story is the foundation the other
three child stories bind to: the Codex adapter implements the port, the CLI
orchestrator dispatches through it, and the acceptance matrix exercises it.

This story delivers the contract surface and pure types only. It does not
migrate Codex behavior onto the port (Unit 2), does not flip the CLI
dispatch (Unit 3), and does not introduce the acceptance matrix (Unit 4). No
existing behavior changes: `CodexAdapter::managed_projection()` is not yet
overridden, so `plan_managed_codex_project_lifecycle` continues to drive
Codex unchanged until Unit 3.

Parent design: `feature-managed-fallback-target-parity` Unit 1.

## Units

- `crates/core/src/managed_projection.rs` (new): `AcquiredProjection`,
  `ManagedProjectionPlan`, `ManagedPluginWrite`, `ManagedFileWrite`,
  `OmittedComponent`, `ManagedProjectionError`. Reference only existing
  public core types (`ArtifactTree`, `Fingerprint`, `Source`,
  `ResolvedRevision`, `RelativeArtifactPath`, `ComponentId`, `EvidenceCode`,
  `NativeId`, `AbsolutePath`, `DirectoryIdentity`, `ComponentDeclaration`,
  `ManagedProjection`).
- `crates/harnesses/src/managed_projection.rs` (new): `ManagedProjectionPort`
  trait, `ManagedAcquisitionContext`, `ManagedProjectionContext`. Re-export
  from `crates/harnesses/src/lib.rs`.
- `crates/harnesses/src/registry.rs` (modified): add the defaulted
  `managed_projection()` accessor to `trait HarnessAdapter`.
- `crates/core/src/lib.rs` (modified): re-export the new module.

The exact signatures are in the parent feature's Unit 1 design body. The
stable error codes carried by `ManagedProjectionError::code()` / `summary()`
must match the existing Codex orchestrator's `ErrorDetail` codes verbatim
(`managed_project_source_missing`, `managed_project_source_unavailable`,
`managed_project_catalog_missing`, `managed_project_catalog_invalid`,
`managed_project_plugin_source_invalid`, `managed_project_plugin_unreadable`,
`managed_project_mcp_invalid`, `managed_project_mcp_conflict`,
`managed_project_drifted`, plus `unsupported_resource_kind` and
`required_unsupported` for the new general cases) so Unit 3's mapping is
one-to-one and user-facing output is byte-identical.

## Implementation notes

- Purely additive: no existing public symbol is removed or renamed. No
  behavior change. `cargo test -p skilltap-core --lib` and `cargo test -p
  skilltap-harnesses --lib` must pass without modifying any existing test.
- `ManagedPluginWrite` / `ManagedFileWrite` intentionally mirror the
  CLI-private `ManagedProjectPluginWrite` / `ManagedProjectFileWrite`
  (`crates/cli/src/application/execution.rs:227-242`) so Unit 3 is a
  mechanical `From` translation. The CLI types stay private; the core types
  become the port's currency.
- `ManagedProjectionContext::kind` is spelled against a placeholder until
  Unit 3 lifts `NativeLifecycleKind`. To keep this story independently
  compilable, define a small `ManagedLifecycleKind` enum in
  `crates/harnesses/src/managed_projection.rs` now (the values Codex uses:
  `MarketplaceAdd`, `MarketplaceRemove`, `MarketplaceUpdate`,
  `PluginInstall`, `PluginRemove`, `PluginUpdate`) and have Unit 3 add the
  `From<NativeLifecycleKind>` conversion at the CLI boundary.
- The port is `Sync` and object-safe: `acquire`/`project` take `&self` and
  `&Context`; the contexts borrow only `&` references. `&'static dyn
  ManagedProjectionPort` is the registry's currency, mirroring the existing
  optional ports.
- Manual `Display`/`Error` impls for `ManagedProjectionError` (this crate
  does not depend on `thiserror`, matching the `ObservationPathError` precedent
  in `registry.rs`).

### Completion

- Execution capability: highest, as directed by the autopilot caller because
  this is a cross-crate public adapter contract consumed by every managed
  fallback target.
- Review weight: standard (caller).
- Files changed: `crates/core/src/managed_projection.rs`,
  `crates/core/src/lib.rs`, `crates/harnesses/src/managed_projection.rs`,
  `crates/harnesses/src/lib.rs`, `crates/harnesses/src/registry.rs`, and this
  story.
- Tests added/removed: added one core table test pinning every
  `ManagedProjectionError::code()` variant, one harness interface test proving
  `&dyn ManagedProjectionPort` object safety and acquisition-to-plan type round
  trip, and one default-accessor test; removed none and did not modify existing
  tests.
- Simplification: reused the existing validated domain/path/source/state types
  directly, kept target codecs out of core, and introduced only the six
  lifecycle variants currently required by Codex.
- Discrepancies from design: `SourceRevisionResolver` is publicly exported from
  `skilltap_core::updates` rather than `runtime`, and `ComponentDeclaration`
  from `skilltap_core::plugin_graph` rather than `domain`; the contexts use
  those existing public homes without changing signatures or dependency
  direction. Added the explicitly required `RequiredUnsupported` error variant,
  which the parent prose and stable-code list require even though its sample
  enum accidentally omitted it. Derived equality for acquired data and plans
  so the required interface round-trip can compare the public values directly.
- Adjacent issues parked: none.
- Dispatch: direct-read only; ownership was bounded to two new modules, their
  re-exports, one default trait accessor, focused tests, and this story, and the
  caller prohibited delegation.
- Verification: `cargo test -p skilltap-core --lib` passed 331 tests;
  `cargo test -p skilltap-harnesses --lib` passed 26 tests; `cargo check -p
  skilltap-core -p skilltap-harnesses`, strict all-target Clippy for both crates,
  `cargo fmt --all -- --check`, and `git diff --check` passed.

## Acceptance criteria

- [ ] `crates/core/src/managed_projection.rs` defines `AcquiredProjection`,
      `ManagedProjectionPlan`, `ManagedPluginWrite`, `ManagedFileWrite`,
      `OmittedComponent`, and `ManagedProjectionError` with the signatures in
      the parent Unit 1 design, referencing only existing public core types.
- [ ] `crates/harnesses/src/managed_projection.rs` defines
      `ManagedProjectionPort`, `ManagedAcquisitionContext`,
      `ManagedProjectionContext`, and `ManagedLifecycleKind` with the
      signatures in the parent Unit 1 design.
- [ ] `HarnessAdapter::managed_projection()` exists and defaults to `None`;
      `CodexAdapter` does not yet override it.
- [ ] An interface test (throwaway test adapter, like the registry contract
      story used) constructs a `ManagedProjectionPort` impl, calls
      `acquire`/`project`, and asserts the round-tripped plan equals the
      inputs — proving object-safety and type round-trip.
- [ ] `ManagedProjectionError::code()` returns the exact existing
      `ErrorDetail` code strings (one assertion per variant).
- [ ] `cargo test -p skilltap-core --lib` and `cargo test -p
      skilltap-harnesses --lib` pass; no existing test changes.

## Out of scope

- Codex relocation onto the port (Unit 2 /
  `feature-managed-fallback-target-parity-codex-adapter`).
- Target-agnostic orchestrator and dispatch flip (Unit 3 /
  `feature-managed-fallback-target-parity-orchestrator`).
- Shared acceptance matrix (Unit 4 /
  `feature-managed-fallback-target-parity-acceptance`).
- Any concrete managed-fallback adapter for a new target.
