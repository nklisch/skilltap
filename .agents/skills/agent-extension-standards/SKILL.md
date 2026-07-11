---
name: agent-extension-standards
description: Current attested Codex, Claude Code, and Agent Skills extension contracts for skilltap. Use when designing, implementing, reviewing, or documenting harness adapters; plugin or marketplace manifests; native plugin lifecycle; skill directories or SKILL.md frontmatter; AGENTS.md/CLAUDE.md bridges; configuration scopes; compatibility transfer; adoption, reconciliation, status, or update behavior.
---

# Agent Extension Standards

Ground extension work in the current research substrate. Do not reuse remembered
Claude-only marketplace assumptions or flatten the two harnesses into a generic
plugin format.

## Load the applicable contract

Read `.research/analysis/briefs/current-agent-extension-standards.md` before
making an architectural or behavioral decision in this domain.

Load a specialist brief when exact native details matter:

- Codex: `.research/analysis/campaigns/marketplace-standards/specialists/codex.md`
- Claude Code: `.research/analysis/campaigns/marketplace-standards/specialists/claude.md`
- Agent Skills: `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

Use attestations under `.research/attestation/` for source-direct verification.
Treat specialist briefs and the parent synthesis as analytical guidance, not
primary sources. Never cite `.research/.import-holding/` as evidence.

## Preserve these boundaries

- Treat a skill as its complete directory with top-level `SKILL.md`. Never
  manage only the Markdown file when sibling resources exist.
- Use `.agents/skills/` as skilltap's canonical portable placement when the
  skill is compatible. Create harness-native links or copies through adapters.
- Keep Codex and Claude plugin manifests, catalogs, identities, caches, scopes,
  consent, and update semantics distinct.
- Prefer deterministic native marketplace and plugin operations. Do not mutate
  a native plugin cache as an installation API.
- Keep `~/AGENTS.md` as skilltap's canonical global instruction file and model
  `~/.codex/AGENTS.md` and `~/.claude/CLAUDE.md` as native bridges, including
  their precedence and divergence hazards.
- Separate desired skilltap state, native declared state, and effective
  installed state.
- Classify transfer as faithful, materializable, partial, or blocked. Surface
  material consequences for piecewise operation-scoped approval.
- Register only sources selected by the user. Do not add search, browsing,
  ranking, recommendation, or inventory-discovery behavior.

## Check freshness

The authoritative brief is dated 2026-07-10 and carries explicit revisit
triggers. When a decision depends on a native command, schema, path, or lifecycle
that may have changed, verify the current official primary source. If the
contract moved, refresh the `.research/` attestations and synthesis before
changing foundation docs or implementation.
