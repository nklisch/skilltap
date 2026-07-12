---
id: story-skilltap-plugin-distribution-release-verification
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-release
depends_on: [story-skilltap-plugin-distribution-release-installer, story-skilltap-plugin-distribution-release-install-surfaces]
release_binding: null
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
