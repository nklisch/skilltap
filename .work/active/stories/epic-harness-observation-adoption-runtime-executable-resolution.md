---
id: epic-harness-observation-adoption-runtime-executable-resolution
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve and Revalidate Harness Executables

Resolve `ConfiguredBinary` path lookups or absolute paths to one canonical
regular executable plus `ExecutableFileIdentity`. Use deterministic explicit
PATH order, reject empty/current-directory components and non-UTF-8 input,
support canonical final symlinks, enforce executable bits, and distinguish not
found, non-file, inaccessible, and non-executable outcomes safely. Revalidate
identity immediately before spawn and report replacement without claiming the
remaining stat/exec race is eliminated.

## Implementation notes

- Files changed: `crates/core/src/runtime/executable.rs`,
  `crates/core/src/runtime/observation.rs`, `crates/core/src/runtime/mod.rs`.
- Added `SystemExecutableResolver` behind the existing behavior port. Absolute
  and explicit PATH lookups resolve to a canonical regular executable with
  Unix device/inode identity and executable-bit validation.
- PATH is validated in full before searching: it must be UTF-8 and contain only
  non-empty absolute directory components. Missing candidates fall through in
  declared order; the first existing candidate is authoritative and cannot be
  bypassed because it is a directory, inaccessible, or non-executable.
- Final symlinks are permitted and canonicalized to their regular targets.
  Revalidation compares canonical path, regular/executable state, and device/
  inode immediately before a caller spawns; its documentation explicitly
  retains the remaining stat/exec race.
- Added a distinct fixed `ExecutableInaccessible` category. All resolution and
  replacement failures remain source/path-free in Debug, Display, and serde.
- Tests cover absolute identity, PATH precedence and missing fallthrough,
  invalid components even after an earlier match, final symlinks, replacement,
  missing/non-file/non-executable/inaccessible distinctions, and secret
  canaries using the shared native-process fixture.
- Discrepancies from design: relative PATH directories are rejected along with
  explicit `.` because accepting them would reintroduce implicit working-
  directory dependence and make resolution nondeterministic.
- Adjacent issues parked: none.

## Review

- Approved after fresh-context review.
- Confirmed explicit PATH validation/order, first-existing-candidate
  authority, absolute/final-symlink canonicalization, regular/executable
  checks, device/inode identity, and last-moment revalidation with the
  documented remaining stat/exec race.
- Focused resolver tests pass 8/8; fixed enum errors and resolver boundaries
  do not expose raw paths.

## Verification

- `cargo test -p skilltap-core runtime::executable --locked`
- `cargo clippy -p skilltap-core --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` (261 tests across workspace suites)
- `cargo doc --locked --workspace --no-deps`
- `cargo build --locked --release -p skilltap`
- `scripts/verify-compiled-binary.sh /storage/cargo-target/release/skilltap`
