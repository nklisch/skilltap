---
id: epic-harness-observation-adoption-contracts-foundation-integration
kind: story
stage: done
tags: [documentation, testing]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-operation-selectors, epic-harness-observation-adoption-contracts-snapshots-ports]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: docs
created: 2026-07-11
updated: 2026-07-12
---

# Align and Verify Observation Foundations

Align SPEC/ARCH/UX/HARNESS-CONTRACTS with compiled-only mutation authority,
scope-bearing stable identity, explicit missing-config enablement, ephemeral
fresh observations, and non-adoptable shared Claude declarations. Add
cross-layer strict wire, old-shape rejection, same-ID multi-scope, artifact
non-alias, and compiled CLI compatibility verification; regenerate public
documentation artifacts when applicable.

## Implementation

- Aligned SPEC, ARCH, UX, and HARNESS-CONTRACTS with exact `ResourceKey`
  identity/selectors, compiled-profile-only mutation authority, narrowing-only
  probes, ephemeral fresh snapshots, persisted provenance/apply state,
  explicit missing-config enablement, safe typed findings/ports, and observable
  but non-adoptable shared Claude declarations.
- Aligned README and public website guides/reference, then regenerated
  `llms-full.txt` twice with byte-identical output and passed the VitePress
  build.
- Corrected the production default policy: both Codex and Claude are disabled
  until explicitly enabled. First-use status reports zero targets, recommends
  `harness enable`, and creates no configuration or state files; explicit
  enabled configuration remains authoritative.
- Added cross-layer verification that fresh declared/effective resources,
  findings, and profile evidence cannot enter state; equal logical IDs remain
  distinct across global/project inventory, state, and plan selectors; and a
  complete normalized snapshot rejects raw native payload channels.
- Shared-Claude execution coverage is intentionally deferred to the Claude and
  adoption features because no native adapter or adoption workflow exists yet;
  this story establishes the authoritative non-adoptable contract.

## Verification

- Locked workspace formatting, warnings-denied all-target Clippy, and all
  workspace/all-target tests pass.
- Foundation integration tests pass 3/3; storage and first-use/explicit-config
  focused suites pass.
- `npm run build` passes in `website/`; repeated LLM documentation generation
  is byte-identical (`c653da723571ec84e97e579ce1fcc3fc768a4ce99d726409196072a50d849a71`).

## Review

- Approved after a fresh-context cross-document and cross-layer review of the
  three implementation commits.
- Confirmed foundation and public wording matches compiled-only authority,
  exact scoped identity/selectors, ephemeral observations, disabled first-use
  defaults, and shared-Claude non-adoption.
- Confirmed 3/3 foundation tests, config and CLI suites, compiled-binary tests,
  deterministic LLM generation, and the website build all pass.
