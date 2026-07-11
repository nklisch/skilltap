---
id: epic-cross-harness-materialization-publish-verification
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: [epic-cross-harness-materialization-publish-transaction]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
