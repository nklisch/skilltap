---
id: epic-harness-observation-adoption-integration-platform
kind: story
stage: done
tags: [testing]
parent: epic-harness-observation-adoption-integration
depends_on: [epic-harness-observation-adoption-integration-cli]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Verify Platform and Failure Guardrails

Cover CODEX_HOME and Claude-home isolation, bounded missing/unknown/malformed/
non-zero/hanging/flood native failures, safe diagnostics, and portable Linux/
macOS path contracts using existing runtime abstractions and barriers.

## Implementation notes

- Added bounded flood-output detection coverage asserting typed failure and
  secret-safe diagnostics.
- Existing runtime integration covers `CODEX_HOME` isolation, repeatable
  bounded process/tree observation, and platform-resolved paths; existing
  native-process fixtures cover hanging and termination behavior.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-harnesses --test detection flood_native_output_is_bounded_and_secret_safe --offline`
- `cargo clippy --workspace --all-targets --offline -- -D warnings`

## Review

Verdict: Approve with comments - bounded platform/failure coverage is green;
native macOS execution remains CI/environment dependent.
