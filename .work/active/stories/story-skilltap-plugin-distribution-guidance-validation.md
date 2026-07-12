---
id: story-skilltap-plugin-distribution-guidance-validation
kind: story
stage: review
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-guidance
depends_on: [story-skilltap-plugin-distribution-guidance-layout, story-skilltap-plugin-distribution-guidance-diagnostics]
release_binding: null
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
