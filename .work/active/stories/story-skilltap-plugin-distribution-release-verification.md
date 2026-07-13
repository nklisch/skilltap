---
id: story-skilltap-plugin-distribution-release-verification
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-release
depends_on: [story-skilltap-plugin-distribution-release-installer, story-skilltap-plugin-distribution-release-install-surfaces]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Gate and verify the versioned release publication

Wire package/guidance validation, compiled help and bootstrap tests, website
build, platform binary verification, checksum generation, and provenance
attestation into CI/release workflows. Provide an offline fixture path that
validates the release manifest consumed by bootstrap without publishing or
touching caller state.

Acceptance criteria:

- CI runs format/lint/tests, package/guidance validation, website build, and
  platform binary checks before release assets publish.
- Release assets, checksums, source identity, and attestations are reproducible
  and consumed by latest-compatible bootstrap resolution.
- Dry-run verification is deterministic, offline, and does not touch credentials
  or native harness caches.

## Implementation notes
- Execution capability: highest; release workflow and provenance gates.
- Review weight: standard (autopilot caller policy).
- Files changed: `.github/workflows/release.yml`.
- Tests added: release validate job runs offline plugin package, installer, and public installation-surface fixtures before asset publication.
- Discrepancies from design: sibling pointer remains an explicit external parity input and is not mutated by CI.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast substrate review at standard weight. Release validation now
runs package, installer, and public installation-surface fixtures before
publication; release identity, checksums, platform assets, and provenance
attestation checks remain gated in the workflow.
