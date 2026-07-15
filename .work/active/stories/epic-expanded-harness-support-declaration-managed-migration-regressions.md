---
id: epic-expanded-harness-support-declaration-managed-migration-regressions
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract, epic-expanded-harness-support-declaration-managed-planner-acknowledgment, epic-expanded-harness-support-declaration-managed-execution-status]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Migrate Existing Profiles and Preserve Regressions

## Checkpoint

Move every current managed/standalone path onto the shared authority and
acknowledgment contract without changing existing Supported behavior or persisted
schemas.

## Design element

- Expand exact compiled profiles to state `skill.install/update/remove`,
  `managed.projection`, `component.skill`, and `component.mcp` independently in
  global and project scope.
- Remove mutation fallback to `Unverified` and delete
  `HarnessAdapter::supports_managed_projection` after all consumers use port +
  profile + declaration contract.
- Migrate Codex, Factory, Gemini, OpenCode, Qwen, and any landed Kiro managed
  planner from adapter-local acknowledgment to unconditional omission evidence.
- Add declaration contracts only where exact documented paths, lossless codec,
  ownership/collision checks, and complete-tree/document surfaces are attested.
  Default-absent contracts remain blocked.
- Migrate standalone global skill and project canonical/link mutation to exact
  profile bindings while preserving shared-content all-target rules and target-
  local link repair/removal.
- Preserve current native-first route selection and applied representation
  pinning. Existing Supported exact profiles must not gain a new `--yes`.
- Keep inventory/state schemas and target state fields unchanged; existing state
  fixtures require no compatibility loader.
- Remove the standalone synthetic partial revision marker only after equivalent
  operation consequence tests pass.

## Acceptance evidence

- Existing config/inventory/state fixtures deserialize and round-trip unchanged;
  schema versions remain stable.
- Existing Supported native and managed workflows retain exact paths, args,
  operation classes, ownership, rollback, target isolation, and idempotence.
- Existing optional-loss/non-strict-skill behavior is represented by validated
  partial operations with equal or stronger consequence coverage.
- Unknown/adjacent versions never write through explicit plugin/skill
  install/update/remove, sync, project links, or daemon, including with `--yes`.
- A managed port without a declaration contract cannot exploit `Unverified`.
- Final greps find no broad support gate, default-unverified mutation fallback,
  adapter acknowledgment branch, or synthetic partial revision marker.

## Ordering constraint

Depends on authority, acknowledgment, and execution/status. It may proceed in
parallel with daemon safety once those shared contracts are stable; integrated
acceptance waits for both.
