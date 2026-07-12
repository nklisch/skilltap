---
id: epic-harness-observation-adoption-adopt-candidates
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-status]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Build Adoption Candidates

Implement the pure effective-observation candidate conversion and typed
adoption decisions. Preserve exact scope-bearing keys, native identity,
fingerprint evidence, source lineage, components, and dependencies. Reject
declared-only, malformed, unresolved, and shared Claude project candidates as
explicit unadoptable decisions; never perform I/O.

## Implementation notes

- Added pure `skilltap_core::adoption` planning over effective normalized
  observations and exact target selections.
- Candidate conversion preserves scope-bearing resource keys, source,
  components, resolved dependencies, native identity, fingerprints, and
  adopted source-harness provenance.
- Existing equivalent inventory entries become `already_managed`; semantic
  differences become conflicts; malformed/unconvertible candidates fail
  without external I/O.

## Verification

- `cargo fmt --all`
- `cargo check -p skilltap-core --all-targets --offline`

The pure candidate story is ready for review; merge, persistence, and CLI
stories remain dependent.

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
