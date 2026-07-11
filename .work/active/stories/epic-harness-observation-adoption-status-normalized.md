---
id: epic-harness-observation-adoption-status-normalized
kind: story
stage: done
tags: [cli,infra]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-status-integration]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Normalize Status Resources and Findings

Complete the observation-backed status contract. Build one typed observation
request/outcome per exact harness and concrete scope, preserve successful
sibling outcomes, and project parsed Codex/Claude resources and registered
findings into deterministic plain/JSON output. Observe documented canonical
instruction, skill, marketplace, plugin, settings, and cache roots through
bounded adapters without scanning arbitrary home/project content. Compare
normalized observations with desired inventory and recorded state so missing,
drifted, unmanaged, unknown-version, and partial state produce actionable
attention findings. Keep status read-only and repeatable.

## Acceptance criteria

- `status` invokes the shared normalization coordinator for every selected
  harness/scope and never drops a healthy sibling when another fails.
- Output includes stable resource identities/kinds and typed health findings,
  not only aggregate filesystem entry counts.
- Canonical `~/AGENTS.md`, `~/.agents/skills`, documented marketplace/plugin
  roots, and project instruction/config paths are included without broad tree
  discovery; all observation limits remain bounded and no native or skilltap
  store is written.
- Desired inventory and recorded state are compared conservatively; drift and
  unknown/observe-only profiles remain attention-required and never imply
  mutation authority.
- Plain/JSON output and exit classes remain derived from one typed result, and
  repeated status leaves bytes, types, links, and mtimes unchanged.

## Implementation notes

- Added named, bounded canonical-root adapters for the global `.agents/skills`
  directory, Codex skills/plugins, and Claude skills/plugins. Project
  observations remain limited to `.agents`,
  `.codex`, or `.claude`; no parent-directory scan is used for `~/AGENTS.md`.
- Reworked CLI native status projection to build one
  `ObservationRequest`/`HarnessObservationOutcome` per reachable harness and
  exact scope, then compose them through `ObservationBatch` and
  `normalize_observations`. A failed outcome does not discard successful
  siblings.
- Typed observed surfaces now expose stable resource identities, resource
  kinds, native-entry counts, profile authority, and observe-only health. The
  unknown-profile finding is surfaced as `capability.unverified` and keeps the
  aggregate at attention-required. Desired and recorded key-set differences
  produce a conservative `resource.drifted` warning without mutation.
- Canonical instruction/settings locations are represented as read-only typed
  instruction resources; their content is not copied or written by status.

## Verification

- `cargo test -p skilltap --all-targets --offline`
- `cargo test -p skilltap-harnesses --all-targets --offline`
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings`
- `cargo fmt --all`

The focused canonical-root test verifies that unrelated home files are not
observed and that named roots remain deterministic. Full status integration
coverage verifies sibling partial failure, typed output, and native-tree
read-only behavior.

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
