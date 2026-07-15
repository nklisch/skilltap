---
id: feature-managed-fallback-target-parity-contract
kind: story
stage: done
tags: []
parent: feature-managed-fallback-target-parity
depends_on: []
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-13
updated: 2026-07-15
---

# Managed Projection Port Contract and Pure Types

## Review (standard, approve — 2026-07-13, convergence on `caf5df03`)

Same-harness fresh-context re-review of the implementor's option-2 correction
at commit `caf5df03`, against the prior bounce record and the ACTUAL existing
`managed_project_error` call sites in `crates/cli/src/application.rs` (audited
directly, not via implementor notes). All three prior findings are resolved;
all focused checks re-ran clean. Approving review → done.

### Material 1 — Codex vocabulary removed from target-neutral defaults

`crates/core/src/managed_projection.rs` `summary()` defaults audited; none
embed Codex vocabulary:

- `CatalogMissing` -> "The selected source has no compatible marketplace
  document." ("Codex" dropped; was "...no Codex-compatible...")
- `PluginMissing` is now `{ detail }`-carrying, so it has no fixed default at
  all (the prior Codex-worded default is gone with the variant shape change).
- `UnsupportedResourceKind`, `RequiredUnsupported`, `SourceMissing`,
  `SourceUnavailable`, `McpConflict` — none mention Codex.
- `grep -ni codex` across both contract modules returns zero matches.

Material 1 resolved.

### Material 2 — variable-summary canonical codes modeled with typed detail

Independent call-site audit of every contract-covered code. "Distinct" below
means distinct user-facing summary strings actually emitted by the existing
Codex orchestrator under that code:

| code | distinct summaries | sites | contract shape |
|---|---|---|---|
| `managed_project_mcp_invalid` | 6 | 2072, 2081, 2117, 2146, 2191, 2225 | `McpInvalid { detail }` ✓ |
| `managed_project_drifted` | 3 | 1922, 2181, 2399 | `Drifted { detail }` ✓ |
| `managed_project_plugin_invalid` | 4 | 1879, 1890, 2311, 2341 | `PluginMissing { detail }` ✓ |
| `managed_project_plugin_source_invalid` | 3 (4 sites; 1614==1623 share text) | 1566, 1599, 1614, 1623 | `PluginSourceInvalid { detail }` ✓ |
| `managed_project_plugin_unreadable` | 2 | 2013, 2316 | `PluginUnreadable { detail }` ✓ |
| `managed_project_catalog_invalid` | 2 | 1463, 1805 | `CatalogInvalid { detail }` ✓ |
| `managed_project_mcp_conflict` | 1 | 2129 | `McpConflict` fixed ✓ |
| `managed_project_source_missing` | 1 | 1453 | `SourceMissing` fixed, byte-exact ✓ |
| `managed_project_source_unavailable` | 1 | 1751 | `SourceUnavailable` fixed, byte-exact ✓ |
| `managed_project_catalog_missing` | 1 | 1813 | `CatalogMissing` fixed (Codex-neutralized, see below) |

Every canonical code with multiple existing summaries carries typed per-instance
`detail: &'static str`; `summary()` returns that detail unchanged. Codes are
variant-owned: `code()` is a `const` match over variants, and `Other { code, ..
}` is the only escape hatch. No variable-summary canonical code was missed; the
model is unambiguous. The regression test
`contextual_summaries_vary_without_changing_the_typed_code` uses byte-exact
strings from real call sites (the 2072 and 2081 summaries). Material 2
resolved.

`McpConflict` confirmed genuinely single-summary (one call site at 2129,
byte-exact match "The existing mcp_servers value is not a table."); correctly
kept as a fixed unit variant.

### `Other` discipline

Documented in two places: the variant doc-comment ("A failure code defined by
one adapter, not an alias for a canonical variant's code") and the Scope
section ("`Other` is reserved for truly adapter-specific codes and must never
reproduce a canonical variant's code"). The discipline prevents silent drift:
no adapter can shadow a canonical code via `Other`, and the regression test
pins the canonical codes by variant. Discipline is in place.

### Minor — accessor lifetime aligned

`registry.rs:123` now reads
`fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort>`
(elided lifetime, borrowing from `&self`), matching the established
optional-port siblings `native_lifecycle`, `instruction_bridge`, and
`skill_projection`. No `'static dyn ManagedProjectionPort` remains. Object
safety is proven by the interface test, which constructs
`&dyn ManagedProjectionPort` and round-trips `acquire`/`project` against a
throwaway adapter. No object-safety regression; instance-bound ports remain
reachable. Minor resolved.

### Focused verification re-run

- `cargo test -p skilltap-core --lib` -> 332 passed.
- `cargo test -p skilltap-harnesses --lib` -> 26 passed.
- `cargo clippy -p skilltap-core -p skilltap-harnesses --all-targets --
  -D warnings` -> no issues.
- `cargo fmt --all -- --check` -> clean.
- `git diff --check` -> clean.
- `cargo check --workspace` -> compiles.
- `CodexAdapter` does not override `managed_projection()` (only the trait
  default at `registry.rs:123` exists), so Codex behavior is unchanged.

### Parked nit (does not block)

The Scope section's sentence "Unit 3 can therefore map both code and
context-specific summary one-to-one and keep diagnostics byte-identical" is
now slightly imprecise for `CatalogMissing` specifically: its canonical
default was deliberately Codex-neutralized under Material 1, so the Unit 2
Codex adapter cannot reproduce the exact legacy "...no Codex-compatible
marketplace document." text through the typed variant without violating the
`Other` discipline. The acceptance criteria explicitly accept the
Codex-neutral canonical summary, so this is a deliberate tradeoff, not a
defect. Unit 2/Unit 2 review should be aware that this one fixed-summary code
intentionally changes user-facing text for Codex users; no contract change
required here.

### Verdict

All material defects (Material 1, Material 2) and the Minor item are
resolved. No variable-summary canonical code was missed; the type model is
unambiguous; `McpConflict` is correctly single-summary; `Other` is
disciplined; the accessor lifetime aligns with object safety preserved.
Approve; advance review → done.

## Review (standard, bounce — 2026-07-13)

Cross-model GLM review of the OpenAI host implementation at commit `1443a1c1`.
Verification re-ran clean: 2 new core tests + 328 existing, 2 new harnesses
tests + 24 existing, clippy `-D warnings` clean, `cargo fmt --check` clean,
`git diff --check` clean, `CodexAdapter` does not override
`managed_projection()` (Codex behavior unchanged), workspace compiles. Two
material contract defects block approval; one minor consistency note.

### Material 1 — Codex leak in target-agnostic contract summaries

`crates/core/src/managed_projection.rs`:

- `CatalogMissing.summary()` -> "The selected source has no **Codex**-compatible
  marketplace document."
- `PluginMissing.summary()` -> "The selected plugin does not contain a valid
  **Codex** manifest and complete component graph."

These strings live in `skilltap-core`, the target-agnostic layer, and the
word "Codex" will surface to users of ANY future adapter (Gemini, OpenCode,
file-managed) that returns these variants. This is precisely the Codex-shape
leak the parent feature's pre-mortem commits to avoiding ("exchanges only
normalized plans ... no path logic crosses the boundary"). The contract is
the wrong layer for target-specific diagnostic vocabulary.

Fix: drop "Codex" from both (e.g., "...has no compatible marketplace
document." / "...does not contain a valid manifest and complete component
graph."). The summaries are not pinned by a test, so this is a one-line edit
per variant with no downstream breakage — but it must land now, before Unit 2
consumes the contract.

### Material 2 — The "byte-identical user-facing output" claim is false

The story body asserts the summaries make Unit 3's mapping one-to-one and
"user-facing output byte-identical." This is not achievable with the
implemented shape. The existing Codex orchestrator emits many DISTINCT
summary strings under the SAME code, and the enum provides exactly ONE summary
per code:

- `managed_project_mcp_invalid` -> 6 distinct summaries
  (`application.rs:2072, 2081, 2117, 2146, 2191, 2225`)
- `managed_project_drifted` -> 3 distinct summaries
  (`application.rs:1922, 2181, 2399`)
- `managed_project_plugin_invalid` -> 4 distinct summaries
  (`application.rs:1879, 1890, 2311, 2341`)
- `managed_project_plugin_source_invalid` -> 3 distinct summaries
  (`application.rs:1566, 1599, 1614/1623`)
- `managed_project_plugin_unreadable` -> 2 distinct summaries
  (`application.rs:2013, 2316`)
- `managed_project_catalog_invalid` -> 2 distinct summaries
  (`application.rs:1463, 1805`)

Unit 3's planned `managed_project_error(error.code(), error.summary())` would
therefore collapse every context-specific summary to the enum's single fixed
text — a user-facing behavior regression — OR force the Codex adapter to
bypass every typed variant via `Other { code, summary }`, which defeats the
typed enum's purpose (the primary adapter would use the escape hatch for
almost every error site).

Fix (pick one, record the choice in the body):

1. Drop the byte-identical claim. State that codes are byte-identical and
   summaries are CANONICAL defaults; adapters use `Other` for context-specific
   summaries. Document the `Other` discipline: `Other` is for adapter-specific
   codes only, never for replicating a canonical code string (silent drift
   otherwise). This is the smallest honest fix.
2. Add a per-call summary override to the variants that carry context (e.g.,
   `McpInvalid { detail: &'static str }`, `Drifted { detail: &'static str }`,
   `PluginInvalid { detail: &'static str }`, `PluginSourceInvalid { detail:
   &'static str }`, `PluginUnreadable { detail: &'static str }`,
   `CatalogInvalid { detail: &'static str }`), keeping `code()` stable. This
   preserves byte-identity at the cost of more verbose variants.

Either resolves the false guarantee; option 1 is the lower-friction contract
for a foundation story and option 2 can be revisited if Unit 2 finds the
escape hatch too coarse.

### Minor — `'static` bound diverges from the established optional-port pattern

`registry.rs`: the three existing optional ports return `Option<&dyn Port>`
(elided lifetime, borrowing from `&self`); the new `managed_projection()`
returns `Option<&'static dyn ManagedProjectionPort>`. The choice is sound for
a stateless port and the Unit 2 design returns `&Self` (a promoted
unit-struct ref), but it is inconsistent with the pattern this story claims
to mirror and forecloses instance-bound ports (e.g., one holding a
detected binary path). Not blocking; align to `Option<&dyn
ManagedProjectionPort>` for consistency unless a recorded reason demands
`'static`.

### Action required

Resolve Material 1 (drop "Codex" from the two summaries) and Material 2
(correct the byte-identical claim per option 1 or 2). Minor is optional.
Re-run `cargo test -p skilltap-core --lib` and `cargo test -p skilltap-
harnesses --lib`; if option 2 is chosen, add per-variant `detail` coverage.
Then return to review.

## Scope

Implement Unit 1 of the managed-fallback-target-parity feature design: the
`ManagedProjectionPort` adapter trait (in `skilltap-harnesses`) and its pure
supporting types (in `skilltap-core`), plus the defaulted
`HarnessAdapter::managed_projection() -> Option<&dyn ManagedProjectionPort>`
accessor. This story is the foundation the other
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

The exact signatures are in the parent feature's Unit 1 design body, as
corrected by this story's review findings. The stable error codes carried by
`ManagedProjectionError::code()` must match the existing Codex orchestrator's
`ErrorDetail` codes verbatim (`managed_project_source_missing`,
`managed_project_source_unavailable`, `managed_project_catalog_missing`,
`managed_project_catalog_invalid`, `managed_project_plugin_invalid`,
`managed_project_plugin_source_invalid`, `managed_project_plugin_unreadable`,
`managed_project_mcp_invalid`, `managed_project_mcp_conflict`,
`managed_project_drifted`, plus `unsupported_resource_kind` and
`required_unsupported` for the new general cases). Canonical variants whose
existing Codex call sites share one code but use multiple summaries carry a
per-instance `detail: &'static str`; `summary()` returns that detail unchanged.
Unit 3 can therefore map both code and context-specific summary one-to-one and
keep diagnostics byte-identical. `Other` is reserved for truly adapter-specific
codes and must never reproduce a canonical variant's code.

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
  `&Context`; the contexts borrow only `&` references. The registry returns
  `Option<&dyn ManagedProjectionPort>`, matching the established optional-port
  pattern and allowing either stateless or adapter-instance-bound ports.
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
  `ManagedProjectionError::code()` variant, one regression test proving two
  exact context-specific summaries retain the same typed canonical code, one
  harness interface test proving `&dyn ManagedProjectionPort` object safety
  and acquisition-to-plan type round trip, and one default-accessor test;
  removed none and did not modify existing tests.
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
  Review established that one canonical summary per code could not preserve
  existing diagnostics, so the six variable-summary canonical variants now
  carry typed per-instance detail while retaining variant-owned codes. The
  accessor lifetime was also aligned with the existing optional-port pattern.
  The call-site audit found only one `managed_project_mcp_conflict` summary, so
  `McpConflict` remains a fixed-summary unit variant.
- Adjacent issues parked: none.
- Dispatch: direct-read only; ownership was bounded to two new modules, their
  re-exports, one default trait accessor, focused tests, and this story, and the
  caller prohibited delegation.
- Verification: `cargo test -p skilltap-core --lib` passed 332 tests;
  `cargo test -p skilltap-harnesses --lib` passed 26 tests; `cargo check -p
  skilltap-core -p skilltap-harnesses`, strict all-target Clippy for both
  crates, `cargo fmt --all -- --check`, and `git diff --check` passed.

## Acceptance criteria

- [x] `crates/core/src/managed_projection.rs` defines `AcquiredProjection`,
      `ManagedProjectionPlan`, `ManagedPluginWrite`, `ManagedFileWrite`,
      `OmittedComponent`, and `ManagedProjectionError` with the signatures in
      the parent Unit 1 design, referencing only existing public core types.
- [x] `crates/harnesses/src/managed_projection.rs` defines
      `ManagedProjectionPort`, `ManagedAcquisitionContext`,
      `ManagedProjectionContext`, and `ManagedLifecycleKind` with the
      signatures in the parent Unit 1 design.
- [x] `HarnessAdapter::managed_projection() -> Option<&dyn
      ManagedProjectionPort>` exists and defaults to `None`; `CodexAdapter`
      does not yet override it.
- [x] An interface test (throwaway test adapter, like the registry contract
      story used) constructs a `ManagedProjectionPort` impl, calls
      `acquire`/`project`, and asserts the round-tripped plan equals the
      inputs — proving object-safety and type round-trip.
- [x] `ManagedProjectionError::code()` returns the exact existing
      `ErrorDetail` code strings (one assertion per variant), while every
      canonical code with multiple existing Codex summaries carries typed
      per-instance detail and returns it unchanged from `summary()`.
- [x] `CatalogMissing` has a target-neutral canonical summary; no canonical
      summary embeds Codex vocabulary. `Other` remains reserved for
      adapter-specific codes rather than aliases of canonical codes.
- [x] `cargo test -p skilltap-core --lib` and `cargo test -p
      skilltap-harnesses --lib` pass; no existing test changes.

## Out of scope

- Codex relocation onto the port (Unit 2 /
  `feature-managed-fallback-target-parity-codex-adapter`).
- Target-agnostic orchestrator and dispatch flip (Unit 3 /
  `feature-managed-fallback-target-parity-orchestrator`).
- Shared acceptance matrix (Unit 4 /
  `feature-managed-fallback-target-parity-acceptance`).
- Any concrete managed-fallback adapter for a new target.
