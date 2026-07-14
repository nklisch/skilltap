---
id: epic-expanded-harness-support-candidate-admission-cursor-boundary
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/cursor-skills.md
  - .research/attestation/cursor-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Validate Cursor Boundaries

## Checkpoint

Close Cursor's Agent Skills path and reload gaps while revalidating the already
attested MCP files in an isolated editor/CLI profile. Record exact source and
native evidence in this story body and conclude `admitted`, `observe_only`, or
`blocked`.

Validation retains `~/.cursor/mcp.json`, `<project>/.cursor/mcp.json`, and the
`cursor-agent mcp list`/tool-observation surface, then establishes exact
documented global/project skill roots, complete siblings and executable intent,
project/global precedence, update/reload visibility, and whether editor and CLI
consume the same promised skills. It also pins exact `cursor-agent` version
output and profile identity, MCP schema/transports, unknown/unowned entry
preservation, ownership-safe update/removal, and OAuth/extension/cache
boundaries.

Create `crates/harnesses/tests/candidate_cursor_boundary.rs` only after all
Cursor roots are redirectable away from operator state.

## Acceptance evidence

- [ ] Exact version bytes and both documented skill roots are reproduced in an
      isolated profile.
- [ ] Whole skill trees remain visible after sibling/content/mode updates and
      obey documented scope precedence in both editor and CLI where promised.
- [ ] Both MCP files preserve unrelated fields/servers, expose same-name
      precedence, and match fresh CLI list/tool state after reload.
- [ ] OAuth and extension registration remain native/user-owned and absent from
      skilltap evidence.
- [ ] Owned removal and immediate repeat are idempotent and cache-independent.
- [ ] Known MCP behavior alone cannot produce `admitted` while the skill
      boundary or editor/CLI equivalence remains unresolved.

## Ordering

Runs after the shared gate and before Cursor's admission checkpoint. It does not
add a registry entry or mutating port.
