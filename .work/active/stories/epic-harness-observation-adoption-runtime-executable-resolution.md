---
id: epic-harness-observation-adoption-runtime-executable-resolution
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve and Revalidate Harness Executables

Resolve `ConfiguredBinary` path lookups or absolute paths to one canonical
regular executable plus `ExecutableFileIdentity`. Use deterministic explicit
PATH order, reject empty/current-directory components and non-UTF-8 input,
support canonical final symlinks, enforce executable bits, and distinguish not
found, non-file, inaccessible, and non-executable outcomes safely. Revalidate
identity immediately before spawn and report replacement without claiming the
remaining stat/exec race is eliminated.
