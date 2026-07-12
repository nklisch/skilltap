---
id: epic-rust-control-plane-domain-contracts-capability-compatibility
kind: story
stage: done
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-identity-scope-source]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Define Capability and Compatibility Evidence

## Scope

Implement Unit 3 from the parent feature: extensible capability identifiers and
support states plus separate behavioral-compatibility and transfer-fidelity
classifications carrying target-specific evidence and consequences.

## Acceptance criteria

- [x] Dotted capability ids are validated without closing the future namespace.
- [x] Capability sets distinguish supported, unsupported, and unverified.
- [x] Behavioral compatibility and transfer fidelity cannot be conflated.
- [x] Every non-faithful result requires machine-readable evidence and a material
  consequence suitable for agent output.
- [x] Representative classifications serialize deterministically and round-trip.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: `crates/core/src/domain/capability.rs`,
  `crates/core/src/domain/compatibility.rs`.
- Tests added: constructor and serde boundary parity; open dotted capability-id
  validation; all three capability support states; deterministic capability-set
  and partial-result JSON; independent compatibility/fidelity axes; mandatory
  evidence and consequences for every non-faithful fidelity; result/evidence
  target agreement; unknown-field and invalid persisted-value rejection.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct implementation in the two isolated domain modules;
  integration boundaries were already established by the completed identity,
  scope, and source story.
- Verification: locked workspace format, check, clippy with warnings denied,
  and tests pass (32 core tests plus all workspace/doc-test targets).

## Review findings (2026-07-11)

- Blocker: `CompatibilityResult` has no target, so a faithful result with empty
  evidence is unscoped and non-faithful evidence can mix harnesses. Add an
  explicit `HarnessId` target to the result/wire/accessor and reject evidence
  for any other target at constructor and serde boundaries.
- Blocker: affected components currently use `ResourceId` values that do not
  identify the new resource-local component graph. After the resource story
  lands `ComponentId`, use it consistently in evidence and consequences.

## Review resolution (2026-07-11)

- Added an explicit `HarnessId` target to `CompatibilityResult`, its validated
  constructor, serde wire, accessor, and deterministic JSON representation.
- Added typed `EvidenceTargetMismatch` rejection before other result invariants;
  raw construction and deserialization tests prove identical enforcement.
- Replaced affected-component `ResourceId` values with the resource graph's
  validated `ComponentId` in both evidence and material consequences.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Initial target-identity and component-selector findings were resolved
in `fdc9f4c`; stable public re-exports landed in `ed65b63`. Story verified by
implement, adversarial contract review, and integrated locked workspace checks;
fast-lane advance.
