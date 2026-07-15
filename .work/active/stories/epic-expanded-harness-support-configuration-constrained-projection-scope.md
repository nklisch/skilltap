---
id: epic-expanded-harness-support-configuration-constrained-projection-scope
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-contract-lock]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Generalize and Gate Managed Projection

## Checkpoint

Make the delivered managed-projection lifecycle operate on one concrete global
or project `Scope`, require an exact verified adapter profile before any
managed/skill mutation, and expose declaration ownership separately from
runtime activation. The relaxed profiles do not add an activation probe: load,
trust, authentication, and reload remain unverified or pending unless a target
has an independently attested observation path.

## Design element

Apply Unit 2 from the parent feature:

- bind `ManagedProjectionContext` to one explicit `Scope` and keep the
  operation-owned, lock-revalidated execution port;
- gate managed projection through compiled `managed.projection` and
  `component.skill` capabilities;
- retain Codex managed fallback at project scope only and preserve its tests;
- let each constrained adapter expose declaration files and complete skill
  trees without claiming native runtime activation.

Declared writes remain journaled when an effective-state gate is unavailable or
attention-required. No probe is read-only by implication: the constrained
adapters simply do not register a probe, so no production process can be
invoked for status or mutation.

## Acceptance evidence

- A fake adapter passes managed install/update/remove at global and project
  scope through one port; unknown versions and unsupported scopes write nothing.
- Codex project behavior, ownership, recovery, rollback, and idempotency are
  regression-identical; Codex global still selects native lifecycle.
- Project skill and managed plugin mutation both honor their compiled capability
  before touching canonical/native paths.
- Reload/trust/auth are never inferred from declaration bytes; they remain
  typed effective health or pending/unverified state, and repeat operations do
  not rewrite correct declared state.
- All mutation remains operation-bound, lock-revalidated, root-confined, and
  target-local.

## Implementation notes

- Execution capability: highest; the authority contract is implemented at the
  shared mutation boundary and consumed by all three targets.
- The registry exposes `kimi`, `vibe`, and `kilo` without changing bootstrap or
  existing native lifecycle selection.
- Verification: `cargo test --workspace --all-targets` passes, including the
  compiled global/project capability paths and no-probe assertions.

## Completion

This story is `done`; the private source planner and concrete adapters consume
this scope/authority contract.
