---
id: epic-expanded-harness-support-candidate-admission-zcode-boundary
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zcode-skills.md
  - .research/attestation/zcode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Validate ZCode Boundaries

## Checkpoint

Identify ZCode's exact native files and deterministic effective observation in
an isolated installation before adding production constants or ports. Preserve
the documented global `~/.zcode/skills` evidence, but do not infer the missing
project skill or MCP filenames from the `.zcode` family name or import UI.

Validation must establish exact detection/version identity; project skill root;
user/workspace MCP files; direct-edit support; copy-versus-symlink behavior;
complete sibling/executable access; global/project precedence; per-skill and
per-server enablement; reload/effective state; unknown-field preservation;
owned update/removal; and cache/credential boundaries on macOS and Linux.

Use `crates/harnesses/tests/candidate_zcode_boundary.rs` only if an official
redirectable host/CLI boundary exists. Record exact sources, commands, bytes,
paths, and results in this story body and conclude `admitted`, `observe_only`,
or `blocked`.

## Acceptance evidence

- [ ] Exact project skill and both MCP files are source-direct, not inferred.
- [ ] Direct writes are proven effective and supported rather than merely
      importable or editable through UI state.
- [ ] Symlink mode preserves the complete skill tree and works with the shared
      canonical project-skill contract; copy mode is not silently substituted.
- [ ] Same-name scope precedence and enablement survive update/removal without
      changing unowned state.
- [ ] Every mutation and reload repeats to no change in isolated roots.
- [ ] Missing filenames, direct-edit authority, or deterministic effective
      observation prevents `admitted` and is retained as the explicit blocker.

## Ordering

Runs after the shared gate and before ZCode's admission checkpoint. It creates no
production adapter or registry entry.
