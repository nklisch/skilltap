---
id: epic-expanded-harness-support-declaration-managed-planner-acknowledgment
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: [epic-expanded-harness-support-declaration-managed-authority-contract]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Plan and Execute Exact Partial Acknowledgments

## Checkpoint

Make declaration-only effective uncertainty and existing compatibility loss
real `OperationClass::Partial` operations, then allow the existing core executor
to run only those partial operations whose exact current acknowledgment was
accepted by a foreground caller.

## Design element

- Construct managed operations from `MutationAuthorization` and adapter
  manifest evidence. `Supported` with no loss stays `SafeMaterialization`;
  declaration uncertainty or optional omission becomes `Partial`.
- Emit stable evidence and material consequences with exact resource/component
  selectors. Declaration uncertainty and omitted behavior remain distinct.
- Add `ExecutionAcknowledgments`, validating operation id plus equality with the
  operation's `AcknowledgmentRequirement`.
- Add `execute_plan_with_acknowledgments`; retain `execute_plan` as the
  empty-acknowledgment safe default.
- Accepted partial operations use the same dependency waves, journal, and
  `ExecutionPort`. Unsupported/conflict operations are never accepted.
- Remove `acknowledged` from `ManagedProjectionContext`. Adapters classify
  optional omissions unconditionally and required loss remains a hard error.
- Foreground `--yes` derives acceptance from the already-built current plan; it
  does not rebuild a safer operation or widen capability support.
- Replace the standalone skill synthetic revision acknowledgment with the same
  operation contract.

## Acceptance evidence

- Preview, sync without `--yes`, and sync with `--yes` contain equal partial
  operations and consequences.
- Empty acknowledgment blocks; exact foreground acknowledgment applies; stale,
  invented, missing, or extra requirements fail validation.
- Accepted partial execution produces applied/no-change disk outcomes while the
  aggregate remains attention-required for the lasting consequence.
- Required unsupported components, drift, conflicts, trust/auth, unknown
  versions, and native `Unverified` remain blocked with `--yes`.
- Independent safe siblings proceed and dependents of a blocked partial skip.
- No blocked attempt writes inventory, state, target files, or a pending journal.

## Ordering constraint

Depends on the authority contract. Execution/status consumes both the
authorization result and exact partial execution semantics.

## Implementation notes

- Added `ExecutionAcknowledgments` and
  `execute_plan_with_acknowledgments`; the legacy executor remains an empty-
  acknowledgment safe default and accepted partials use the same dependency,
  journal, and execution-port path.
- Added a validated partial managed-materialization constructor that preserves
  exact evidence, consequence, resource/component selectors, and attention
  semantics.
- Managed lifecycle planning now computes declaration/optional-loss partials
  from the current plan rather than rejecting them in the adapter gate, and
  foreground lifecycle execution derives acknowledgment from that unchanged
  plan. Daemon execution still uses the empty default.
- Unknown or absent capability lookup now fails closed instead of becoming
  `Unverified`.
- Verification: `cargo fmt --all && cargo test --workspace --all-targets`
  (704 passed).

## Completion

Implemented and verified. Remaining lock-time binding, standalone-skill
migration, daemon classification, and status projection are subsequent
checkpoints.
