---
id: story-skilltap-plugin-distribution-cli-diagnostics
kind: story
stage: implementing
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-cli-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Boundary-aware, secret-safe diagnostics

Normalize parser and runtime failures into the existing outcome contract. The
deepest recognized command boundary must be identified without echoing invalid
values or raw clap/native payloads. Plain errors remain on stderr, JSON errors
remain one stdout document, and next actions point at the relevant `--help`.

Acceptance criteria:

- Parse failures identify a recognized boundary, stable error code, and safe
  help next action; unknown commands fall back to root help.
- Native argv, stdout/stderr, credentials, environment values, and dynamic
  parser messages never appear in rendered plain or JSON diagnostics.
- Schema version, result classes, exit mapping, and existing `--yes` behavior
  remain unchanged.
- Unit tests cover missing command, invalid target/source/scope, non-UTF-8
  arguments, fake native failure output, and rendering fallback behavior.
