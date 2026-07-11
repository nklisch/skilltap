---
id: epic-harness-observation-adoption-codex-paths
kind: story
stage: review
tags: [infra]
parent: epic-harness-observation-adoption-codex
depends_on: [epic-harness-observation-adoption-detection, epic-harness-observation-adoption-runtime]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/codex.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Derive Codex Observation Paths

Define the bounded Codex global and one canonical project input roots from
`CODEX_HOME`, scope policy, and documented native contracts. Resolve only
configured files/directories, reject unsafe roots, preserve `~/AGENTS.md` and
project override precedence, and never create or scan paths.

## Implementation

- Added `CodexObservationPaths` and `codex_observation_paths` to derive
  CODEX_HOME, global `~/AGENTS.md`, project `AGENTS.md`, and
  `AGENTS.override.md` inputs for one exact scope without filesystem access.
- Added isolated path-policy coverage using explicit HOME/XDG/CODEX_HOME
  environments and a canonical project scope.

## Verification

- Harness Clippy and all seven detection/Codex path tests pass in the locked
  offline workspace.
