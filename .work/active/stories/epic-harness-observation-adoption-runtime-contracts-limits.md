---
id: epic-harness-observation-adoption-runtime-contracts-limits
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Observation Runtime Limits and Ports

Add harness-neutral non-zero bounded request/limit/status contracts with hard
compile-time ceilings and checked cross-field relationships, behavior
ports for executable resolution, process execution, JSON decoding, and external
tree observation, plus a closed safe error taxonomy. Custom Debug/Display and
serde forms must never expose argv, environment values, native output, parser
excerpts, file bytes, or caller-provided raw paths. Keep concrete I/O out of the
contract module and avoid redefining installation or snapshot domain types.
Reject zero, hard-maximum overflow, allocation/counter/duration overflow, a
combined process cap below either stream cap, and a total tree cap below its
per-file cap. Cap JSON nesting at a documented stack-safe maximum and test
every limit at zero and maximum minus/at/plus one.

## Implementation notes

- Files changed: `crates/core/src/runtime/observation.rs`,
  `crates/core/src/runtime/mod.rs`.
- Added pure behavior ports for executable resolution/revalidation, bounded
  native process execution, strict JSON decoding, and external tree
  observation. Requests reuse domain executable/path types and contain no I/O.
- Added non-zero process, JSON, and tree limits with hard compile-time ceilings,
  checked byte conversions, a documented stack-safe JSON depth maximum, and
  cross-field invariants for combined output and tree counters/bytes.
- Added safe closed runtime errors and process statuses. Sensitive requests,
  process output, decoded JSON, and external tree payloads have redacted custom
  `Debug` implementations and intentionally have no serde surface.
- Tests cover every limit at zero and hard maximum minus/at/plus one,
  cross-field failures, strict serde validation, deterministic tree snapshots,
  secret canaries, non-zero exit status, and fake-port composition.
- Review correction: bounded process, decoded JSON, tree entry, and tree
  snapshot construction is crate-private. Process and tree builders enforce
  their supplied limits before returning public read-only values, including
  checked combined/total byte sums. Tree entries now expose a public kind and
  accessors over private payload fields. Added distinct safe termination and
  post-termination drain failure categories plus direct bypass regression tests.
- Discrepancies from design: none after incorporating the coordinating design
  review's hard-ceiling correction before implementation completed.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core runtime::observation --locked`
- `cargo clippy -p skilltap-core --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` (243 tests across workspace suites after review correction)
- `cargo doc --locked --workspace --no-deps`

## Review

- Initial review found public construction paths that could forge unbounded
  process, JSON, and tree results plus missing termination/drain categories.
- Correction `5167b63` made bounded payload construction crate-private and
  limit-validating, privatized tree payloads, added direct bypass regressions,
  and split termination from post-termination drain failure.
- Fresh re-review approved the corrected contracts; focused tests pass 7/7 and
  the coordinated workspace passes 243 tests, formatting, and strict Clippy.
