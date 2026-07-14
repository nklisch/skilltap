---
id: epic-expanded-harness-support-configuration-constrained-projection-scope
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-contract-lock]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Generalize and Gate Managed Projection

## Checkpoint

Make the delivered managed-projection lifecycle operate on one concrete global
or project `Scope`, require an exact verified adapter profile before any
managed/skill mutation, and add one bounded read-only activation probe used by
post-apply and status observation.

## Design element

Apply Unit 2 from the parent feature:

- change `ManagedProjectionContext.project` to `scope` and add
  `ManagedProjectionPort::supports_scope` plus `activation_probe`;
- replace `managed_project_lifecycle()` with port-owned scope support;
- rename project-only CLI execution types/functions to scope-neutral managed
  projection names without changing revalidation, rollback, or residual rules;
- add profile detection independent of `NativeLifecycleVector`, using compiled
  `managed.projection` and `component.skill` capabilities;
- add typed activation identities/states and registered reload/auth findings;
- let `AdapterObservationPaths` carry bounded authored findings and merge them
  with profile evidence in status;
- retain Codex managed fallback at project scope only and preserve its tests.

Declared writes remain journaled when a trusted/auth/reload gate leaves the
native effective state attention-required. The probe is read-only and cannot
grant mutation capability.

## Acceptance evidence

- A fake adapter passes managed install/update/remove at global and project
  scope through one port; unknown versions and unsupported scopes write nothing.
- Codex project behavior, ownership, recovery, rollback, and idempotency are
  regression-identical; Codex global still selects native lifecycle.
- Project skill and managed plugin mutation both honor their compiled capability
  before touching canonical/native paths.
- Reload/trust/auth are typed effective health, not drift; repeat operations do
  not rewrite correct declared state.
- All mutation remains operation-bound, lock-revalidated, root-confined, and
  target-local.

## Ordering

Depends on the locked profile/probe evidence. The private source planner and all
three concrete adapters depend on this scope/authority contract.
