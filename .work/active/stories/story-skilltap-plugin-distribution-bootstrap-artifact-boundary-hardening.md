---
id: story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening
kind: story
stage: review
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Close bootstrap artifact redirect and publication races

Review follow-up for `story-skilltap-plugin-distribution-bootstrap-artifacts`.

The hardened artifact boundary still needs to enforce release-host policy over
the complete redirect chain and make publication/rollback identity-safe. A
single final `curl %{url_effective}` check is too late to reject an
intermediate cross-host hop. The stat-then-rename publication window and the
unconditional rollback renames can also overwrite a destination that changed
after the last identity check.

Acceptance criteria:

- Every fetched redirect remains on the attested GitHub release hosts, or the
  fetch is rejected before any untrusted response can be accepted.
- Publication uses an identity-safe no-clobber strategy; a destination
  replacement between observation and publish is reported as
  `DestinationChanged` and leaves the unrelated file untouched.
- Post-publish identity failure and parent-sync failure roll back only when the
  destination still has the expected published identity; rollback never
  overwrites an unrelated replacement.
- Isolated test-support fixtures cover cross-host redirects, oversized and
  symlink payloads, checksum/permission/interruption cleanup, destination
  replacement races, and post-publish rollback preservation.

## Review origin

Fresh-context review of the hardened bootstrap artifact commits `c880496` and
`85b56ea` found redirect-chain enforcement and identity-safe rollback gaps.

## Implementation notes
- Execution capability: highest; release transport and publication are security-sensitive filesystem/process boundaries.
- Review weight: standard (autopilot caller policy).
- Files changed: `crates/core/src/runtime/artifact.rs`.
- Tests added: Linux no-clobber first-install, replacement exchange, and rollback preservation cases.
- Discrepancies from design: redirect validation was already applied at every bounded hop; publication now uses Linux `renameat2` no-replace/exchange primitives and preserves replacements observed during rollback.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: supported macOS publication still falls back to overwrite-capable
`fs::rename`; no-prior rollback has a stat-then-unlink race that can remove an
unrelated replacement -> `story-skilltap-plugin-distribution-bootstrap-artifact-portable-rollback-safety`

**Important**: the required isolated redirect-hop, permission/interruption,
and full cleanup/publication regression matrix is still not present ->
`story-skilltap-plugin-distribution-bootstrap-artifact-portable-rollback-safety`

**Nits**: none

**Notes**: Substrate review at standard weight, fresh-context focused security
pass. `cargo test -p skilltap-core runtime::artifact --offline`, workspace
formatting, and clippy with warnings denied pass. Linux no-replace/exchange
tests preserve replacements, but the non-Linux branches at
`crates/core/src/runtime/artifact.rs:745-763` use ordinary rename and can
clobber a destination. The `None` rollback branch at lines 691-697 can unlink
after its identity check. Existing integration fixtures cover bounded and
symlink resolver payloads plus basic checksum/install behavior, but do not
cover redirect-hop rejection, permission/interruption cleanup, or the complete
race/rollback matrix required by this story. Item remains at `stage:
implementing` pending the follow-up.
