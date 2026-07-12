---
id: epic-harness-observation-adoption-runtime-external-tree
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Observe External Trees Without Following Links

Add a bounded descriptor-relative external tree observer separate from managed
artifact APIs. Traverse directories deterministically, read bounded regular
files, and report symlinks with bounded opaque targets without following them.
Snapshots are non-Serialize and Debug-redacted; opaque file/target bytes remain
inside the owning adapter and cannot enter errors, findings, state, or output.
Reject FIFO/socket/device, non-UTF-8, raced, over-depth, over-entry, per-file,
and total-byte cases while walking. Verify parent/name/file identity before and
after open/read using deterministic barriers/fault injection, add an
identifier-valid secret target canary, and execute portable errno behavior
natively on Linux and macOS.

## Implementation notes

- Files changed: `crates/core/src/runtime/external_tree.rs`,
  `crates/core/src/runtime/mod.rs`.
- Added `SystemExternalTreeObserver`, a read-only Unix adapter that opens the
  absolute root component-by-component and every descendant descriptor-relative
  with no-follow flags. It bounds directory enumeration before allocation,
  bounds file and link reads during I/O, and revalidates descriptor and path
  identities after reads and recursive traversal.
- Tests cover deterministic directory/file/live-and-dangling-link snapshots;
  depth, entry, file, total, and link limits; FIFO/socket and non-UTF-8
  rejection; missing/file/link roots; injected permission failure; pre-open,
  post-read, and root replacement races; and secret-safe Debug/error output.
- Linux and macOS errno and filesystem identity shapes are selected with native
  cfgs; native CI remains the portability execution gate.
- Discrepancies from design: deterministic private hook points inject boundary
  failures and replacements directly rather than relying on timing or chmod.
- Dispatch: one highest-effort implementation worker was attempted, then the
  bounded module was completed inline after its patch collided with a
  concurrent runtime export.
- Adjacent issues parked: none.

## Review

- Approved after fresh-context review.
- Confirmed root/component `openat(O_NOFOLLOW)` traversal, sorted bounded
  enumeration, descriptor-relative stat/open/readlink, before/opened/after
  identity checks, incremental file/link/total limits, and special/non-UTF8
  rejection without blocking.
- Confirmed non-serializable redacted payloads, fixed safe errors, and the five
  focused adversarial tests. Core Clippy and workspace formatting pass.
