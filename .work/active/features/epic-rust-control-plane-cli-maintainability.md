---
id: epic-rust-control-plane-cli-maintainability
kind: feature
stage: drafting
tags: [refactor, testing]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# CLI Maintainability

## Brief

Reduce structural and test-infrastructure pressure revealed by the completed
CLI shell without changing grammar, output bytes, channels, exits, storage or
filesystem effects, public identities, test identities, or release behavior.

## Evidence

- `crates/cli/tests/compiled_binary.rs` locally owns an isolated-machine and
  compiled-runner framework assigned by architecture to
  `skilltap-test-support`; application tests duplicate temporary-root cleanup,
  and `bare_help.rs` duplicates a compiled assertion without honoring the
  release-binary override.
- `StatusApplication::execute` is a 122-line orchestration combining document
  loading/classification, scope resolution, target resolution, and projection.
- `output.rs` mixes 210 production lines with 167 inline test lines while the
  other CLI modules use private sidecars.
- CI/release repeat the compiled verification command, and one compiled test
  hardcodes `3.0.0` instead of using the workspace version contract.

The cadence scan leaves the declarative 499-line Clap grammar, exhaustive
79-line transitional dispatch, typed outcome builders, and independent
parser-vs-binary command-tree coverage unchanged.

<!-- The refactor design pass will define implementation units. -->
