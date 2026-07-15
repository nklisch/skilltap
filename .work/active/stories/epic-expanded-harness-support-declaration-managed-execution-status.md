---
id: epic-expanded-harness-support-declaration-managed-execution-status
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract, epic-expanded-harness-support-declaration-managed-planner-acknowledgment]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Revalidate Declarations and Separate Effective Status

## Checkpoint

Carry exact profile authority through the existing managed execution ports and
make status report owned declared bytes separately from effective harness state.
No new executor or persisted runtime-health state is introduced.

## Design element

- Replace the single-capability `ConfiguredAdapterProfile` fallback with an
  exact `MutationProfileBinding` containing executable identity, native version,
  scoped profile, required capabilities, and `MutationAuthorization`.
- Attach profile bindings to managed plugin, global skill, and project canonical/
  link entries. Re-detect and compare the complete binding under the lock before
  any file/tree/link mutation.
- Retain root-confined reads/writes, exact expected identities/fingerprints,
  target-local state seeds, pending attempts, disk verification, rollback, and
  residual reporting.
- Inspect an existing managed plugin declaration through the source-free pure
  `ManagedProjectionPort::plan(Remove)` path and recorded prior manifest;
  inspect standalone skills through bounded complete-tree observation.
- Correct adapter filesystem roots/settings from coarse `Effective` rows to
  `Declared` rows.
- Produce `Effective` only from a deterministic bounded
  `EffectiveStateProbePort`. Declaration-managed resources without such proof
  expose explicit effective-unverified attention.
- Preserve trust, auth, disabled, failed, and reload-required results as
  effective health rather than declaration drift.
- Keep state schema unchanged; fresh profile/probe evidence determines current
  verification status.

## Acceptance evidence

- Profile/executable/scope/component authority and every disk expectation are
  revalidated under lock; changed or unknown versions fail before mutation even
  with `--yes`.
- Supported and declaration-managed paths have identical owned write/update/
  remove, target-isolation, rollback, pending-retry, and immediate-repeat disk
  behavior.
- Correct declaration bytes appear healthy only in the `Declared` layer.
  Effective-unverified never renders loaded/healthy.
- Positive effective health requires the existing bounded probe; interactive or
  side-effectful commands and caches are never used.
- Unmanaged collisions, ambiguous precedence, malformed documents, literal
  secrets, drift, and replacement races preserve every unrelated byte.
- Plain and JSON output derive from one outcome and distinguish declared,
  effective, drift/conflict, trust/auth, and unverified states.

## Ordering constraint

Depends on authority and acknowledgment. Daemon safety and migration consume
this exact binding/status behavior.

## Implementation notes

- Managed lifecycle entries now retain exact required capabilities, managed
  surface kinds, declaration authorization, and the scoped adapter declaration
  contract. The executor re-detects the configured executable/profile and
  recomputes that authority under the lock before checking file/tree identities.
- Missing mutation capabilities remain unsupported rather than becoming
  unverified, and declaration-contract changes invalidate the planned write.
- Status keeps normalized native observations suitable for adoption while its
  rendered native surfaces are explicitly labeled `layer=declared`; owned
  unverified managed declarations produce separate declared-healthy and
  effective-unverified attention output with no persisted schema change.
- Verification: `cargo fmt --all && cargo test --workspace --all-targets`
  (704 passed).

## Completion

Implemented and verified. Daemon exclusion and profile migration consume the
lock-time binding and status semantics from this checkpoint.
