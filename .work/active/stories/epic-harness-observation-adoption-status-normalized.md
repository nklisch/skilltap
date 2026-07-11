---
id: epic-harness-observation-adoption-status-normalized
kind: story
stage: implementing
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
