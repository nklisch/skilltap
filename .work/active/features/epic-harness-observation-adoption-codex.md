---
id: epic-harness-observation-adoption-codex
kind: feature
stage: done
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-detection]
release_binding: 3.0.0
research_refs:
  - .research/analysis/campaigns/marketplace-standards/specialists/codex.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Codex Observation Adapter

Implement read-only Codex observation for global and one canonical project
scope. Use configured `CODEX_HOME`, compiled profile/probe evidence, documented
marketplace/config/skill/instruction paths, and cache/manifests as effective
evidence only. Preserve declared versus effective plugin state, whole skill
directories and conformance/loadability, project trust, layered configuration,
and `AGENTS.override.md` precedence. Structured/native drift and malformed
siblings become safe findings; no cache/config write, marketplace browsing,
interactive plugin action, or guessed install/update behavior is allowed.

## Design

The Codex adapter composes the detection registry, `PlatformPaths::codex_home`,
bounded external-tree observation, and shared observation contracts. It supports
global and exactly one canonical project scope, never scans unrelated paths,
and keeps `~/AGENTS.md` as the global instruction source. Declared config is
read from documented Codex TOML/skill/plugin locations; cache and manifests are
effective evidence only and never write APIs.

The adapter emits layered declared/effective resources, trust and project
override findings, whole-directory skill conformance, and safe malformed
siblings. Native bytes remain inside the adapter boundary; normalized records
carry typed identities, ownership, source, and findings only. Missing,
malformed, and partially unreadable siblings are retained independently.

## Design decisions

- **Scope**: global and one canonical project are supported; `--all-scopes`
  composition belongs to the later normalizer/status feature.
- **Instruction precedence**: `AGENTS.override.md` is an effective project
  override while root `AGENTS.md` remains the declared canonical document.
- **Effective state**: caches/manifests confirm loadability and version basis
  only; they never create desired resources or imply install authority.

## Implementation units

1. `epic-harness-observation-adoption-codex-paths` — derive bounded Codex
   global/project roots and documented file inputs — depends on `[detection,
   runtime]`.
2. `epic-harness-observation-adoption-codex-config` — parse strict config,
   trust, marketplace, and malformed sibling evidence — depends on
   `[epic-harness-observation-adoption-codex-paths]`.
3. `epic-harness-observation-adoption-codex-resources` — observe layered
   plugins, whole skills, instructions, and effective cache evidence — depends
   on `[epic-harness-observation-adoption-codex-config]`.
4. `epic-harness-observation-adoption-codex-integration` — verify global,
   project, precedence, malformed sibling, no-mutation, and safe-output
   behavior — depends on `[epic-harness-observation-adoption-codex-paths,
   epic-harness-observation-adoption-codex-config,
   epic-harness-observation-adoption-codex-resources]`.

## Acceptance criteria

- Codex observation is bounded, deterministic, read-only, and limited to global
  plus one canonical project scope.
- Declared/effective plugin state, whole skill directories, instruction
  precedence, trust, malformed siblings, and cache evidence are represented in
  shared typed observations without native payload leakage.
- No cache/config/marketplace write or resource scanning occurs; focused and
  workspace tests pass on Linux and native macOS behavior jobs.

## Implementation

- Completed all four Codex stories: bounded path derivation, config evidence,
  complete skill/instruction tree observation, and integration verification.
- `skilltap-harnesses` now exposes read-only Codex inputs and snapshots over
  the shared runtime contracts while preserving global/project precedence and
  safe native boundaries.

## Verification

- Nine harness detection/Codex tests, 16 fixtures, 211 core tests, workspace
  Clippy, and locked integration suites pass. No native config/cache/state
  writes occur.

## Review

- Aggregate review approved from the green child records and locked workspace
  ladder; fresh macOS execution remains CI-gated by the project contract.
