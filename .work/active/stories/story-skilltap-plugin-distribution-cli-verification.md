---
id: story-skilltap-plugin-distribution-cli-verification
kind: story
stage: done
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-cli-contract
depends_on:
  - story-skilltap-plugin-distribution-cli-help-contract
  - story-skilltap-plugin-distribution-cli-diagnostics
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Compiled CLI contract and reference parity

Exercise every public help path through the compiled binary in isolated
test-support roots, then align `website/reference/cli.md` and any UX wording to
the executable contract. The website remains a conceptual index and links
agents to `skilltap --help`; it does not become a second command grammar.

Acceptance criteria:

- Every root, group, and leaf `--help` invocation exits 0, writes only stdout,
  and includes purpose, usage, relevant flags, and exit guidance.
- Representative invalid invocations prove documented plain/JSON channels,
  result classes, exit codes, boundary labels, next actions, and redaction.
- Website reference covers all command families, scope/target model,
  acknowledgment semantics, JSON envelope, and links to executable help without
  duplicating full syntax.
- Tests use `skilltap-test-support` isolated homes and fake native processes;
  no host Codex/Claude configuration is touched.

## Implementation notes
- Execution capability: highest available local capability; compiled CLI and documentation parity are release-facing surfaces.
- Review weight: standard (autopilot project default).
- Files changed: `crates/cli/tests/compiled_binary.rs`, `website/reference/cli.md`, `docs/UX.md`.
- Tests added: all 26 compiled leaf help paths plus plain/JSON invalid boundary and redaction scenarios in isolated machines.
- Discrepancies from design: the website now links agents to executable help through invocation examples and a conceptual family index instead of duplicating a static grammar.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context review. The compiled suite exercises root,
group, and all 26 leaf help paths plus representative plain/JSON invalid
boundaries in isolated machines; the full workspace test suite and clippy are
green. Website and UX references remain conceptual, identify all ten current
families, link agents to executable help, and document the stable result,
scope/target, and generic partial-acknowledgment contracts without becoming a
second grammar. The website `--yes` wording was corrected during review to
match the executable semantics.
