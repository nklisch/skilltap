---
id: feature-relaxed-target-harness-research
kind: feature
stage: done
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

## Research execution

Completed 2026-07-12 using current official product documentation and the ARD
source-bound research discipline.

### Method

- Reversed the prior marketplace/lifecycle admission gate rather than silently
  rewriting its historical conclusion.
- Rechecked every prior candidate and exclusion for complete skill directories,
  user/project MCP, supported write surfaces, observation, reload, and policy
  boundaries.
- Expanded the survey to Amp, Cursor, Cline, Windsurf/Devin Desktop Cascade,
  Zoo Code, ZCode, and the retired Roo Code line.
- Sought disconfirming evidence for missing project MCP, core-vs-extension MCP,
  retired products, interactive-only state, undocumented paths, trust, OAuth,
  and reload constraints.
- Authored source-direct attestations before synthesis and retained the prior
  report as the superseded historical record.

### Findings

- Tier A: Factory Droid, Qwen Code, GitHub Copilot CLI, Gemini CLI, Junie,
  Kimi Code CLI, OpenCode, Kilo Code, Mistral Vibe, Kiro CLI, and Amp.
- Boundary-spike candidates: Cursor, Zoo Code, and ZCode.
- Conditional compound target: Pi plus a skilltap-managed MCP adapter package.
- Excluded: Goose, Windsurf Cascade, and Cline lack an attested ambient project
  MCP surface; Roo Code is shut down.
- Hooks, agents, commands, LSP, apps, and other components are compatibility
  warnings/partial outcomes rather than admission requirements.

### Outputs

- `.research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md`
- `.research/reference/harness-candidates/INDEX.md` entries 27–55
- `.research/attestation/` source-direct attestations for the revised gate

### Verification

`lint-citations.py` resolved 46 citations with 0 broken, 0 thin, and 0 pattern
flags. The attestation-tier audit reported no findings.
