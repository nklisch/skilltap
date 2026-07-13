---
id: story-skilltap-plugin-distribution-guidance-layout
kind: story
stage: done
tags: [content]
parent: epic-skilltap-plugin-distribution-guidance
depends_on: [story-skilltap-plugin-distribution-guidance-core]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Document skilltap configuration and instruction bridges

Add the progressively loaded configuration and instruction references under
`plugin/skills/skilltap/references/`. Ground every path and precedence claim in
the current foundation docs. Explain global/project scope, state-file roles,
managed artifact ownership, canonical `~/AGENTS.md`, Codex/Claude bridges, and
drift handling without teaching an agent to overwrite native content blindly.

Acceptance criteria:

- `references/configuration.md` accurately describes the XDG skilltap folder,
  policy/inventory/state/managed roles, and scope flags.
- `references/instructions.md` accurately describes canonical AGENTS content,
  native bridge paths, precedence, and divergence health findings.
- References preserve complete skill-directory semantics and remain linked from
  the core skill without paths escaping its root.
- No reference introduces shared repository metadata, discovery, or a second
  configuration schema.

## Implementation notes
- Execution capability: standard; documentation must remain foundation-grounded.
- Review weight: standard (autopilot caller policy).
- Files changed: `plugin/skills/skilltap/references/configuration.md`, `plugin/skills/skilltap/references/instructions.md`.
- Tests added: existing offline package validation passes with linked complete references.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast substrate review at standard weight. Configuration/state roles,
scope/target behavior, complete skill-directory ownership, canonical
AGENTS.md, native bridge precedence, and drift-preserving guidance match the
foundation contracts. Offline package validation passes.

## Review follow-up (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Corrected the Codex native instruction paths in the reference to
honor `${CODEX_HOME:-$HOME/.codex}`, including the higher-precedence
`AGENTS.override.md`, matching the foundation contract. No other layout or
precedence claims changed.
