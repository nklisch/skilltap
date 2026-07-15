---
id: epic-expanded-harness-support-declaration-managed-authority-contract
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: []
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Define Exact-Profile Declaration Authority

## Checkpoint

Create the pure mutation ceiling used by native lifecycle, managed plugin, and
complete-skill planning. The result must derive only from a verified compiled
profile, exact concrete scope, required operation/component capabilities, the
concrete managed surface set, and an explicit scoped adapter declaration
contract.

This checkpoint owns no CLI mutation. It establishes the contract that later
stories consume.

## Design element

- Add `CapabilityProfileSelection::mutation_support` without a fallback:
  unknown, verified-observe-only, and absent capabilities return `None`.
- Add `MutationChannel`, `ManagedSurfaceKind`,
  `ManagedDeclarationContract`, `CapabilityRequirement`,
  `MutationAuthorization`, and `authorize_mutation` in core.
- Expose the existing component-kind-to-capability mapping from
  `core::compatibility` as the single registry for `component.*` ids.
- Add default-absent `HarnessAdapter::managed_declaration_contract(scope)`.
- Prepare explicit profile construction for independent `skill.*`,
  `managed.projection`, `component.skill`, and `component.mcp` support.
- Do not yet remove existing call-site gates; migration occurs after consumers
  compile against this contract.

## Acceptance evidence

- Exact `VerifiedCompiled` + all `Supported` authorizes native and managed
  channels.
- Any native `Unverified` blocks.
- Managed `Unverified` authorizes only non-empty complete-tree/managed-document
  surfaces covered by the exact scope's declaration contract.
- Any missing/`Unsupported` requirement, unknown/observe-only profile,
  uncovered surface, or wrong scope blocks.
- Mixed skill/MCP requirements retain exact affected component ids.
- Runtime narrowing cannot widen authority.
- Core and harness unit tests pass with no production behavior change yet.

## Ordering constraint

This story is the foundation. Planner acknowledgment, execution/status, and
profile migration depend on its exact semantics.

## Implementation notes

- Added `CapabilityProfileSelection::mutation_support`, which returns no
  mutation evidence for unknown/observe-only profiles or omitted capabilities.
- Added the pure `mutation_authority` module with exact channel rules,
  per-scope capability requirements, managed surface coverage, and explicit
  declaration-contract opt-in.
- Exposed the compatibility component-kind registry as the shared capability
  mapping and added the adapter contract hook while retaining the old route
  gate for the subsequent migration checkpoint.
- Verification: `cargo fmt --all && cargo test -p skilltap-core -p
  skilltap-harnesses` (514 passed).

## Completion

Implemented and verified. The next checkpoint may consume the authority result;
no mutation route was widened in this checkpoint.
