---
id: epic-harness-observation-adoption-adopt-merge
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-candidates]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Merge Adoption Decisions

Add conservative equivalence, cross-harness coalescing, conflict isolation,
stable adopted provenance, deterministic ordering, and inventory merge helpers.
Preserve manual/unrelated desired resources and make repeated merges a logical
no-op.

## Implementation notes

- Added conservative `equivalent_candidates` semantics over exact resource key,
  kind, source, complete component graph, and resolved dependency set. Native
  identities and fingerprints remain revalidation evidence only.
- Updated `plan_adoption` to group exact keys deterministically, isolate
  semantic conflicts, and coalesce equivalent cross-harness candidates with a
  union target set and lexicographically stable adopted origin.
- Added `merge_inventory`, preserving existing resources and policy while
  accepting equivalent repeated additions, recording project scopes, and
  rejecting same-key semantic conflicts.
- Added focused tests for evidence-insensitive equivalence, stable coalescing,
  idempotent inventory merge, and conflict rejection.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core adoption::tests --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
