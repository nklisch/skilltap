---
id: story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening
kind: story
stage: implementing
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
