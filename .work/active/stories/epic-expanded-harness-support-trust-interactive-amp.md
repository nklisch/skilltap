---
id: epic-expanded-harness-support-trust-interactive-amp
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-trust-interactive-contract-lock]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Amp Adapter

## Checkpoint

Implement Unit 3 from the parent feature: one distinct Amp adapter with exact
version-bounded managed skills/MCP, declared-versus-effective workspace trust,
and faithful preservation of behavior-bearing skill-local MCP.

## Units

- Add `trust_interactive/amp.rs` with `AmpAdapter`, `AmpSkillProjection`, and
  the contract-locked bounded `AmpEffectiveStateProbe`.
- Add `trust_interactive/amp_projection.rs` with `AmpSettingsDocument`,
  `AmpMcpPlacement`, and `AmpManagedProjection`.
- Register/export Amp only after the contract-lock story pins the user settings
  path, precedence, profile, doctor output, and skill-local semantics.
- Consume the shared source normalizer, exact-scope projection/execution,
  target-local state, and project-skill service.

## Contract constraints

- Project `.agents/skills` is canonical and produces no redundant link/tree.
  Global managed skills use documented `~/.agents/skills`; other supported
  roots are observed for precedence/conflicts, not synchronized as copies.
- Edit only owned `amp.mcpServers` members in the locked user or nearest project
  settings document. Preserve unknown settings and unowned servers.
- Keep a skill-owned `mcp.json` inside its complete skill tree when relative
  paths or lazy activation are behavior-bearing; never duplicate it into scoped
  settings. Record exact skill and server fingerprints for drift/removal.
- `amp mcp doctor`, workspace trust, and auth state are read-only evidence.
  They never mutate trust, enter persistent policy, or widen compiled authority.
- Untrusted correct config stays declared/owned and attention-required, not
  drifted or repeatedly rewritten.

## Acceptance evidence

- Known/unknown profiles and both-scope managed capability routing.
- Amp project canonical no-link behavior and global single-root complete-skill
  lifecycle.
- User/workspace settings precedence, preserving merge/remove, same-name
  conflict, drift, target isolation, rollback, and repeat idempotency.
- Trusted healthy effective observations and untrusted/auth/failed declared-only
  outcomes with stable secret-safe findings.
- Skill-local relative-path/lazy MCP update and removal without a duplicate
  settings entry.
- Optional omissions require acknowledgment; required unsupported behavior
  blocks under `--yes`.

## Ordering

Consumes the locked native contract. The final acceptance story waits for this
and the Junie checkpoint; child verification advances directly to done without
a separate review pass.
