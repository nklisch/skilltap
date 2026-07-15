---
id: epic-expanded-harness-support-configuration-constrained-source
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-projection-scope]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Normalize Portable Source Components Privately

## Checkpoint

Implement the family-private selected-plugin/source layer used by Kimi, Vibe,
and Kilo projection ports. It extracts one explicit plugin, validates complete
skills with the review-ready Agent Skills contract, and normalizes MCP
transport/auth/reference semantics without defining a universal plugin format.

## Design element

Create:

- `crates/harnesses/src/adapters/configuration_constrained/mod.rs`
- `crates/harnesses/src/adapters/configuration_constrained/source.rs`

Implement the parent feature's `SelectedPortablePlugin`, `PortableMcpServer`,
`PortableRemoteTransport`, and `AuthenticationRequirement` shapes. Reuse
`CodexPluginGraphReader`, `ClaudePluginGraphReader`, `ValidatedSkillTree`, and
`validate_agent_skill`. Resolve only the explicitly selected catalog entry or
direct plugin root; never recursively discover candidates. Reject symlinks and
literal credential values at the bounded source boundary.

The source layer classifies data only. Each concrete target decides which
normalized server is faithful and encodes its own native document privately.

## Acceptance evidence

- Explicit local/Git selectors produce one deterministic graph and complete
  skill trees; missing, ambiguous, malformed, escaping, or recursively found
  candidates fail closed.
- Stdio/remote transport, auth requirement, enablement, timeout, tool filters,
  and credential references survive normalization; secret values do not enter
  state/findings.
- Required malformed/incompatible components block; optional unsupported
  components retain exact omission evidence.
- No Kimi/Vibe/Kilo path or wire vocabulary appears in core or CLI.

## Implementation notes

- Execution capability: high; normalization is bounded and target-neutral while
  each adapter retains its own codec and precedence rules.
- Source credentials are accepted only as references; literal secrets and
  source-relative command paths fail closed before target planning.
- Verification: source normalization tests pass for stdio, remote transport,
  OAuth classification, and literal-header rejection.

## Completion

This story is `done`. Kimi, Vibe, and Kilo consume the normalized source
contract without placing target-specific wire vocabulary in core or CLI.
