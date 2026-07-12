---
id: epic-harness-observation-adoption-contracts-snapshots-ports
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-storage-wires, epic-harness-observation-adoption-contracts-findings, epic-harness-observation-adoption-contracts-installation-profiles]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Define Ephemeral Snapshot and Adapter Ports

Add one-concrete-scope requests, harness observations, partial environment
snapshots, safe adapter errors, and behavior ports for harness adapters and the
shared coordinator. Bind installation/profile evidence to one executable and
expose normalized resources/findings only; no native DTO, I/O, persistence, or
CLI dependency enters core.

## Implementation

- Added an immutable reachable installation/profile evidence envelope that
  stores one exact executable identity and native version. Unreachable
  installations cannot form requests; unknown versions remain valid for
  observation and retain observe-only authority.
- Added exact harness/scope targets and one-concrete-scope requests, plus
  deterministic duplicate-rejecting batches.
- Added context-validated harness observations. Every normalized resource and
  safe finding must match the request's exact harness and scope; declared and
  effective siblings and unresolved native dependency evidence remain valid.
- Added a closed, source-free adapter error vocabulary and explicit observed or
  failed outcomes. Partial environments bind to their originating batch,
  require exactly one outcome per request, reject unexpected/mismatched
  outcomes, and retain successful siblings when another target fails.
- Added adapter and coordinator behavior ports whose signatures expose only
  normalized core values. No native DTO, process/filesystem operation,
  persistence, or CLI dependency entered core.
- Covered strict wires, executable/profile evidence, observe-only unknown
  versions, same-ID multi-scope outcomes, mixed success/failure aggregation,
  missing/duplicate/mismatched targets, foreign resource/finding contexts,
  deterministic ordering, safe-error canaries, and fake behavior ports.

## Verification

- `cargo test -p skilltap-core domain::observation --locked` — 7 passed.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --all-targets --locked` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --workspace --locked` — 223 tests passed.
- `cargo doc --workspace --no-deps --locked` — passed.
- `cargo build --workspace --release --locked` — passed.
- `scripts/verify-compiled-binary.sh /storage/cargo-target/release/skilltap` —
  passed, including 6 compiled-binary integration tests.

## Review

- Approved after a fresh-context review of the complete snapshot and port
  boundary.
- Confirmed unreachable evidence is unrepresentable, unknown versions remain
  observe-only, every resource/finding matches its exact request context, and
  environments require one request-identical outcome for every batch target.
- Confirmed deterministic strict wires, retained mixed success/failure
  siblings, source-free adapter errors, and no I/O, native DTO, persistence,
  runtime, or CLI dependency in the core port surface.
