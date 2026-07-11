---
id: epic-harness-observation-adoption-integration-core
kind: story
stage: done
tags: [testing,correctness]
parent: epic-harness-observation-adoption-integration
depends_on: [epic-harness-observation-adoption-integration-fixtures]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Adoption Core Seams

Exercise equivalent and conflicting cross-harness candidates, declared-only and
unresolved observations, partial siblings, lock contention, manual inventory
edits, stale identity/fingerprint revalidation, and repeat no-op publication.

## Implementation notes

- Added explicit declared-only adoption coverage and a fail-fast lock-contention
  test to the core adoption seam suite.
- Existing pure/storage tests cover semantic coalescing, conflict isolation,
  stale evidence rejection, one-write publication, and repeat idempotence.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core adoption::tests --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
