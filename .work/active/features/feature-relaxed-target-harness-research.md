---
id: feature-relaxed-target-harness-research
kind: feature
stage: drafting
tags: [research]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-candidates-2026-07-12.md
research_origin: operator-request-2026-07-12
research_dials:
  scope_authority: pre-registered
  verification_rigor: full
  intent: survey-landscape
  output_kind: landscape-brief
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reassess target harnesses under the skills-plus-MCP contract

## Brief

Refresh the target-harness candidate analysis using a materially lower but
still faithful admission bar. A candidate needs documented whole-directory
skill loading and MCP support with inspectable global and project behavior.
It does not need a native marketplace, package manager, or plugin lifecycle;
skilltap may own those through documented load paths and managed artifacts.

For every candidate, attest skill directories, MCP configuration and loading,
global/project scope, observable state, reload/update behavior, instruction
support when present, and optional component capabilities such as hooks. Report
whether skilltap can safely own install/update/removal without cache mutation,
which components are faithful or partial, and what adapter tests would be
required. Revisit every previously excluded harness rather than promoting only
the earlier near misses.

## Research questions

- Which popular and less-known harnesses load complete Agent Skills directories
  and configure MCP servers at global and project scope?
- Which documented filesystem/configuration surfaces are supported write APIs
  rather than opaque caches?
- Can skilltap observe effective load state and reconcile managed artifacts
  idempotently without a native lifecycle?
- Which optional components—hooks, instructions, agents, commands, tools, or
  connectors—exist, and which portability consequences must be disclosed?
- Which candidates support faithful skills plus MCP today, and in what adapter
  implementation order?

## Completion

- Refresh or supersede the existing candidate report in accordance with the
  research correction/reversal discipline.
- Create source-direct attestations before citing new claims.
- Include contradiction and disconfirming analysis.
- Produce a capability matrix, candidate tiers, exclusions, adapter posture,
  and acceptance-test matrix.
- Run citation lint with zero broken or thin citations.
