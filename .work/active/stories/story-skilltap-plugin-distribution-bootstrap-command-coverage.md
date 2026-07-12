---
id: story-skilltap-plugin-distribution-bootstrap-command-coverage
kind: story
stage: review
tags: [infra, testing, security]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Restore isolated bootstrap command acceptance coverage

Review follow-up for `story-skilltap-plugin-distribution-bootstrap-command`.

The production command now rejects ambient release/artifact/version fixture
overrides, but the hardened follow-up removed the compiled bootstrap tests
instead of replacing them with a test-only composition seam. Restore
deterministic fixture injection outside the shipped command path and exercise
the public result contract without network or operator state.

Acceptance criteria:

- Isolated compiled tests cover first install, same-major no-op, same-major
  update, blocked major upgrade, `--allow-major`, target narrowing, absent
  harnesses, mixed success/attention, and failed pre-publish preservation.
- A wrong-version/non-executable release and post-publish identity failure are
  reported as attention while the prior binary remains intact.
- Tests inject release metadata/artifacts only through a test-only boundary;
  no production environment variable or arbitrary source argument bypasses
  the canonical HTTPS resolver.
- Plain and schema-1 JSON results retain separate binary and per-harness
  statuses and actionable next actions across every covered branch.

## Review origin

Fresh-context review of the hardened bootstrap command commits `c880496` and
`85b56ea` found the acceptance matrix unrepresented after fixture tests were
removed.

## Implementation notes
- Execution capability: highest; bootstrap combines release transport, executable probing, and atomic publication.
- Review weight: standard (autopilot caller policy).
- Files changed: `crates/cli/src/entrypoint.rs`, `crates/cli/Cargo.toml`.
- Tests added: isolated compiled command composition seam covering first install, same-major no-op/update, major blocking and opt-in, valid-checksummed wrong-version/non-executable releases, post-publish identity rollback preservation, target narrowing, absent/mixed harness outcomes, and plain/schema-1 JSON contracts.
- Discrepancies from design: production execution still constructs only canonical HTTPS system ports; deterministic tests inject ports through a private in-process composition boundary.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: the acceptance matrix is only partially represented in the
private binary helper tests; target narrowing, absent/mixed harness outcomes,
and plain/schema-1 JSON result/next-action contracts are not exercised, and
the wrong-version fixture currently fails checksum verification before the
identity check (this item)
**Important**: the compiled-binary suite still exercises `bootstrap` only for
help/grammar, not an isolated command execution with independent per-target
results (this item)
**Nits**: none

**Notes**: Standard substrate review of `a86e9fc`. The test-only resolver,
fetcher, and installer composition seam is correctly private and the shipped
path still constructs only canonical system ports. The two unit tests pass and
cover install/no-op/same-major update/major block/major opt-in plus a
post-publish rollback publisher. However, they call only
`execute_binary_bootstrap_with`; none invokes the public bootstrap composition
or verifies target selection, absent harnesses, mixed success/attention,
separate binary/per-harness statuses, or plain and schema-1 JSON next actions.
The purported wrong-release case hashes `wrong payload` while fetching
different bytes, so it stops at checksum failure rather than proving a validly
checksummed wrong-version or non-executable release is reported as identity
attention. Keep the story at `stage: implementing` until those deterministic
branches are covered without reintroducing production overrides.
