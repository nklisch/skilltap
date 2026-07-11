---
id: epic-harness-observation-adoption-normalization
kind: feature
stage: done
tags: []
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-codex, epic-harness-observation-adoption-claude]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Native Normalization and Health

Coordinate enabled adapter observations across concrete scopes, retaining
successful siblings when one harness/scope fails. Establish deterministic
marketplace, plugin, skill, and instruction lineage without mutable revisions;
correlate declared/effective instances conservatively; preserve unresolved
dependencies and malformed siblings as findings; and associate cross-harness
resources only from a common declared source plus compatible semantics or an
explicit mapping. Same names, similar URLs, or equal copied fingerprints never
prove equivalence. Return an ephemeral normalized snapshot for status, adoption,
and later reconciliation consumers.

## Design

Normalization consumes successful sibling observations from the Codex and Claude
adapters and returns an ephemeral, deterministic snapshot. It never writes
state and never discards healthy siblings because another harness/scope failed.
Marketplace, plugin, skill, instruction, and dependency lineage remain typed;
declared/effective instances correlate only when a common declared source and
compatible semantics prove equivalence. Names, URLs, copied fingerprints, and
cache coincidence are insufficient.

Unresolved dependencies and malformed siblings remain findings attached to the
surviving observation. Scope and qualified identity are preserved exactly, and
cross-harness associations are explicit rather than inferred. The normalized
snapshot is the single input for later status and adoption operations.

## Design decisions

- **Failure isolation**: retain every successful harness/scope sibling and
  attach typed failure findings for the rest.
- **Equivalence**: require common declared source plus compatible semantics;
  never correlate by name, URL, or copied bytes alone.
- **Lineage**: preserve declared/effective layers, unresolved dependencies,
  malformed siblings, scope, and qualified native identity in the normalized
  result.

## Implementation units

1. `epic-harness-observation-adoption-normalization-graph` — compose enabled
   adapter observations into deterministic scope/harness/layer graphs —
   depends on `[epic-harness-observation-adoption-codex,
   epic-harness-observation-adoption-claude]`.
2. `epic-harness-observation-adoption-normalization-correlation` — implement
   conservative source/semantics equivalence and declared/effective lineage —
   depends on `[epic-harness-observation-adoption-normalization-graph]`.
3. `epic-harness-observation-adoption-normalization-findings` — retain failures,
   malformed siblings, unresolved dependencies, and deterministic health
   findings — depends on `[epic-harness-observation-adoption-normalization-graph]`.
4. `epic-harness-observation-adoption-normalization-integration` — verify
   deterministic repeat normalization, cross-harness non-equivalence, partial
   sibling success, scope preservation, and safe output — depends on
   `[epic-harness-observation-adoption-normalization-graph,
   epic-harness-observation-adoption-normalization-correlation,
   epic-harness-observation-adoption-normalization-findings]`.

## Acceptance criteria

- Normalization is ephemeral, deterministic, read-only, and preserves exact
  harness/scope/layer identity.
- Cross-harness associations require common declared source and compatible
  semantics; names, URLs, copied fingerprints, and caches do not prove it.
- Healthy siblings survive failures, malformed/unresolved evidence remains
  visible, and all normalized output is safe and typed.

## Implementation

- Completed graph composition, conservative source/semantics correlation,
  failure-preserving health summaries, and integration verification over the
  Codex/Claude observation outputs.
- Normalization remains ephemeral and read-only; domain contracts retain exact
  scope/harness/layer identity and reject missing/unexpected sibling outcomes.

## Verification

- Harness tests, normalization tests, 211 core tests, workspace Clippy, and
  locked integration suites pass with deterministic safe output.

## Review

- Aggregate review approved from all green child records and the locked
  workspace ladder.
