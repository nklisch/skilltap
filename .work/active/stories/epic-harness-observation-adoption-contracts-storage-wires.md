---
id: epic-harness-observation-adoption-contracts-storage-wires
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Reset Inventory and State Wires to Exact Keys

Migrate inventory/state maps, resource state, schema errors, managed ownership,
strict goldens, repositories, and storage integration to nested scope-bearing
keys. Keep independent schema constants at 1 for unreleased clean-break v3;
reject old ResourceId-only shapes with no migration or compatibility path.

This story is the coordinated compile bridge for the resource-graph and
managed-ownership migrations. It may implement while those siblings await
review; none of the three can be approved until the combined strict workspace
ladder passes.

## Implementation notes

- Files changed: inventory/state storage tests, deterministic inventory TOML
  and state JSON goldens, storage integration fixtures, and this item.
- Exact wire contract: inventory desired resources and dependencies serialize
  nested `ResourceKey` values; state resources use `key`; managed owners use
  the same nested exact key. Independent schema constants remain `1`.
- Tests added: old sibling `id`/`scope`, state `resource_id`, and string managed
  owner shapes are rejected; equal IDs in global and project scopes coexist in
  inventory and state with distinct managed paths.
- Integration: all document repositories and the atomic managed-artifact/state
  workflow now use exact keys and repeat without extra changes.
- Discrepancies from design: none. The coordinated bridge reused the exact-key
  in-memory maps and schema errors landed with managed ownership, then reset
  every stale caller and persisted fixture without a migration path.
- Adjacent issues parked: none.
- Verification: locked format, workspace all-target check, warnings-denied
  Clippy, 209 workspace tests, warnings-denied rustdoc, optimized release build,
  binary smoke, and six compiled-binary tests all pass.

## Review

- Approved in the coordinated exact-key migration review.
- Confirmed strict rejection of the retired inventory, state, and managed-owner
  wire shapes; no migration or compatibility path remains.
- Confirmed equal logical IDs coexist across global and project inventory and
  state maps while retaining distinct managed artifact ownership.
