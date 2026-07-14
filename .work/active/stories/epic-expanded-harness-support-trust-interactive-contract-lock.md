---
id: epic-expanded-harness-support-trust-interactive-contract-lock
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/junie-skills.md
  - .research/attestation/junie-mcp.md
  - .research/attestation/junie-extensions.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Lock Junie and Amp Native Contracts

## Checkpoint

Implement Unit 1 from the parent feature. Capture exact, bounded native evidence
for one installed Junie release and one installed Amp release before either
adapter receives a mutation-authorized profile or enters the canonical registry.

This story consumes the scope-generic managed/probe/default-binary contract from
`epic-expanded-harness-support-file-managed-contracts`. It must amend that
shared owner if a missing cross-target capability is discovered; it must not
fork a trust-interactive-only runtime or lifecycle port.

## Units

- Add `crates/harnesses/src/adapters/trust_interactive/contracts.rs` with the
  exact `VerifiedTrustInteractiveContract`, `VerifiedMcpContract`, and
  `EffectiveProbeContract` types from the parent design.
- Add isolated bounded version, config, precedence, and effective-state fixtures
  under `crates/harnesses/tests/fixtures/trust_interactive/{junie,amp}/`.
- Pin Junie's exact binary/version contract, scoped MCP schema, and whether any
  deterministic non-TTY effective-state surface exists beyond `/mcp`.
- Pin Amp's exact binary/version contract, selected user settings path, nearest
  project settings precedence, `amp.mcpServers` shape, `mcp doctor` output, trust
  states, and skill-local `mcp.json` behavior.

## Contract constraints

- Do not invent version literals, argv, output grammar, user paths, or trust
  behavior from remembered product behavior.
- A runtime probe is read-only, direct-argv, bounded, explicit-cwd, and
  exact-version decoded. Raw stdout/stderr, config bytes, URLs, secrets, and
  parser text cannot enter findings.
- `InteractiveOnly` is a blocker for MCP mutation/effective support. It is not
  authority to run a pseudo-TTY, parse a cache, or call configured files
  effective.
- Fixtures use inert credential references and test-support-owned HOME/XDG/
  project roots only.

## Acceptance evidence

- Known exact bytes decode to one verified profile; malformed, control-character,
  extra-document, adjacent, and unknown versions cannot grant mutation.
- Both scopes' path/schema/precedence contracts and unknown-field preservation
  are fixture-pinned.
- Effective fixtures distinguish loaded, inactive/disabled, trust-required,
  authentication-required, failed, and unverified states.
- Amp skill-local MCP relative-path/lazy behavior is proven separately from
  scoped settings.
- A target whose minimum contract cannot be locked remains unregistered and
  blocks its dependent adapter story without weakening its sibling.

## Ordering

Depends on the shared file-managed contract checkpoint. Junie and Amp adapter
stories consume this exact evidence and may proceed independently only after it
is complete.
