---
id: story-skilltap-plugin-distribution-guidance-validation
kind: story
stage: done
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-guidance
depends_on: [story-skilltap-plugin-distribution-guidance-layout, story-skilltap-plugin-distribution-guidance-diagnostics]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Validate the complete guidance artifact

Extend offline plugin package validation to assert the complete skilltap skill
directory, strict portable frontmatter, sibling reference links, and durable
no-search guidance boundaries. Keep package validation separate from release
version/source parity and website/install checks.

Acceptance criteria:

- Missing/malformed/detached `SKILL.md` or an escaping/missing reference fails
  validation without mutating the package.
- Both native package manifests expose one complete skill directory with all
  references, preserving channel-specific metadata.
- Tests reject stale marketplace discovery/recommendation language and a
  duplicated leaf command grammar while allowing links to executable help.
- Validation remains offline, deterministic, and repeatable.

## Implementation notes
- Execution capability: highest; package validation is a publication boundary and must reject unsafe trees.
- Review weight: standard (autopilot caller policy).
- Files changed: `crates/cli/tests/plugin_package.rs`.
- Tests added: required guidance references, complete-tree links, and rejection of duplicate command/discovery language.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast substrate review at standard weight. Package validation now
requires complete guidance references, rejects unsafe/missing trees, and
guards against duplicate command grammar and discovery instructions. The
offline plugin package suite passes all four tests.

## Review follow-up (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Follow-up review closed the explicit link-boundary gap: validation
now walks every local Markdown link in `SKILL.md`, rejects missing or
non-regular targets, and rejects absolute/parent-directory escapes. The added
missing-target and escaping-target fixtures pass with the deterministic
offline package suite.
