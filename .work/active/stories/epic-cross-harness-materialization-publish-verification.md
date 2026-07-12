---
id: epic-cross-harness-materialization-publish-verification
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: [epic-cross-harness-materialization-publish-transaction]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Verify Effective Harness Loads

Verify each published projection through a fresh bounded Codex or Claude
observation and record managed ownership only after successful verification.

Acceptance criteria:

- Verification compares normalized identity and fingerprint from the effective
  load path, never a harness cache.
- Verification failures remain typed attention results and do not become owned
  state.
- Verified entries publish one atomic state refresh preserving apply history.

## Implementation Notes

- Added core `LoadVerifier`, `verify_observed_load`, `verify_publication`, and
  typed verification errors. Verification requires an effective, healthy
  observation whose resource, harness, and fingerprint match the publication.
- Added the harness-side `EffectiveObservationVerifier`, which consumes only a
  fresh normalized observation snapshot and never inspects caches.
- Publication receipts now retain verified native identities for the later
  state refresh boundary.
- Verification: core and harness tests plus clippy passed.

## Review Record

- Inline deep review: **pass**. Ownership can be based on a verified effective
  observation, and mismatch/absence remains an explicit typed failure.
