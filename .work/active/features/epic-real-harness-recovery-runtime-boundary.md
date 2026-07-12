---
id: epic-real-harness-recovery-runtime-boundary
kind: feature
stage: drafting
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Detect and isolate real harness processes

## Brief

Make the harness process boundary work with current real Codex and Claude
executables. Detection must parse each documented version form, preserve an
explicit minimal environment containing the configured home and harness roots,
resolve `CODEX_HOME` and `CLAUDE_CONFIG_DIR` consistently, and report the actual
failing boundary when detection cannot complete.

This feature owns blocker inventory entries 1, 3, 4, and 8. It keeps newly
recognized versions observe-only until the native-lifecycle feature corrects
the command surface and grants attested capabilities. It does not change native
marketplace/plugin vectors or resource-state semantics.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: foundation feature for real native lifecycle validation.

## Foundation references

- `docs/ARCH.md` — capability detection and native command execution.
- `docs/HARNESS-CONTRACTS.md` — detection and mutation-authority rules.
- `docs/SPEC.md` — validation, status, and mutation safety.
