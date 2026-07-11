---
id: epic-standalone-skill-lifecycle-compatibility
kind: feature
stage: done
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-tree]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Classify Skill Compatibility

Evaluate one complete skill tree against Codex and Claude loadability contracts
without changing authored content or treating a warning as faithful
equivalence.

## Design

- Parse only documented frontmatter fields; retain unknown authored fields in
  the tree and report malformed metadata as evidence.
- Distinguish strict Agent Skills conformance from harness loadability.
- Required incompatibility blocks; optional unsupported metadata is an exact
  partial consequence requiring foreground acknowledgment.
- Unknown harness versions remain observe-only and never grant mutation
  authority.

## Acceptance

Compatibility findings are target-bound, deterministic, redacted, and usable
by the existing operation acknowledgment contract.

## Implementation notes

Added `skilltap_core::skill_compatibility`, a conservative target-bound
frontmatter classifier that distinguishes strict Agent Skills conformance,
loadability, warnings, and blocked malformed/absent metadata without rewriting
the complete skill tree.

## Review

### Verdict

Approve with comments.

### Findings

- Harness profile adapters must refine loadability for runtime versions and
  translate warnings into exact operation consequences before mutation.

### Verification

Focused compatibility tests and strict core clippy pass.
