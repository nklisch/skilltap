---
id: story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete first-party harness bootstrap contract coverage

Review follow-up for `story-skilltap-plugin-distribution-bootstrap-harness`.

The adapter now has the intended canonical Claude source, qualified plugin
identity, Codex unsupported result, read-first observation, and executable
identity binding. It still needs operation-specific capability authority and
the fake-binary contract suite promised by the feature design.

Acceptance criteria:

- Marketplace registration is attempted only when the selected verified
  profile grants `marketplace.register` for the requested scope; plugin
  installation separately requires `plugin.install`.
- `crates/harnesses/tests/bootstrap.rs` uses isolated fake binaries to assert
  exact Claude marketplace/plugin vectors, user scope, canonical source,
  qualified identity, target isolation, present/missing/unknown observations,
  Codex unsupported behavior, and no cache writes.
- Tests cover capability narrowing, malformed version/list output, and an
  executable replacement between detection and mutation; replacement blocks
  the native mutation.

## Review origin

Fresh-context review of the hardened bootstrap harness commits `c880496` and
`85b56ea` found the marketplace capability check and promised fake-binary
coverage missing.

## Implementation notes
- Execution capability: highest; harness lifecycle writes require native capability and executable identity authority.
- Review weight: standard (autopilot caller policy).
- Files changed: `crates/harnesses/src/bootstrap.rs`, `crates/harnesses/tests/bootstrap.rs`.
- Tests added: isolated fake-binary vectors for present/missing/malformed resources, malformed-version InvalidVersion mapping, unknown-version capability narrowing, target isolation/no-cache-write, Codex unsupported behavior, and executable replacement between detection and mutation.
- Discrepancies from design: operation-specific capability checks are explicit while preserving fail-closed behavior for profiles that cannot attest either operation.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: malformed version output is still collapsed to
`SetupReason::NotInstalled` (`crates/harnesses/src/bootstrap.rs:113-127`), so
the promised malformed-version contract is neither covered nor truthful; add
a fake version-output case and preserve an invalid/unknown-version result
before approving setup (this item)

**Important**: the fake-binary suite does not exercise a narrowed verified
profile withdrawing `marketplace.register` or `plugin.install`, does not
assert that no skilltap cache files are written, and its target-isolation
assertion only runs the Claude adapter in isolation. Add deterministic seams
or fixtures for those acceptance branches (this item)

**Nits**: none

**Notes**: Standard substrate review of `63d8bdb` at highest implementation
capability with standard review weight. The six harness integration tests and
all harness crate tests pass. They now prove Claude's canonical source,
qualified identity, user-scoped vectors, present / missing / malformed-list
handling, Codex unsupported behavior, and executable replacement blocking. The
operation-specific guard itself is fail-closed for known profiles, but no test
can currently withdraw either global capability; the malformed-version mapping
remains a production diagnostic defect. Keep the item at `stage: implementing`
until the missing acceptance evidence and truthful version result are addressed.

## Review (2026-07-12, current contract)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review of `dc8fd63` at highest implementation
capability with standard review weight. The adapter now maps malformed version
JSON to `InvalidVersion`, narrows unknown versions to observe-only, and the
fake-binary suite proves capability narrowing, target isolation, no cache
writes, exact Claude user-scoped vectors/canonical source/qualified identity,
presence and malformed-list handling, Codex unsupported behavior, and
replacement blocking. The two pre-existing detection tests were stale after
the intentional diagnostic correction and were aligned in `23ca090`. Full
`skilltap-harnesses` tests pass; advancing this follow-up to `stage: done`.
