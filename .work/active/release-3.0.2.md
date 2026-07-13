---
id: release-3.0.2
kind: release
stage: quality-gate
tags: []
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Release 3.0.2

## Bound items

- `epic-skilltap-plugin-distribution` and its 39 completed descendants —
  self-hosted Claude/Codex plugin package, verified binary bootstrap, CLI help
  contract, primary plugin installation story, website parity, and release
  automation. These completed items were late-bound from the prior workflow.
- `epic-real-harness-recovery-and-adapter-expansion` — complete native
  lifecycle, state recovery, instruction repair, bootstrap, diagnostics, and
  adapter-eligibility research bundle, including 23 completed descendants that
  were late-bound with their parent.
- `story-agent-directed-usage-docs` — agent-directed human usage guidance.
- 11 release-gate remediation stories — confined and bounded project I/O,
  exact recovery/publication evidence, unsupported-plugin blocking, native
  retry and instruction repair coverage, rollback residual reporting, and
  bounded cleanup.
- `gate-patterns-3.0.2` — four recurring implementation patterns codified.

Total non-release items: **77** (2 epics, 11 features, 64 stories). There were
no unbound archived stubs.

## Gate runs

- **gate-security** (2026-07-12) — 2 findings (1 high, 1 medium), both fixed and reviewed.
- **gate-tests** (2026-07-12) — 6 gaps (3 critical, 2 high, 1 medium), all fixed and reviewed.
- **gate-cruft** (2026-07-12) — 2 medium findings, both removed and reviewed.
- **gate-docs** (2026-07-12) — 8 findings: six corrected; two future-facing
  product promises retained and scoped as implementation features.
- **gate-patterns** (2026-07-12) — 4 patterns codified; 1 rollback adoption
  defect fixed and reviewed.

## Verification

- Full workspace tests and all-target/all-feature Clippy are green.
- Website build, release contract, installer, install-surface, plugin package,
  and cutover checks are green.
- Security, tests, cruft, docs, and patterns release gates run before tagging.
