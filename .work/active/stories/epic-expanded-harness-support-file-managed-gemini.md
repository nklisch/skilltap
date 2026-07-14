---
id: epic-expanded-harness-support-file-managed-gemini
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Gemini CLI Adapter

## Checkpoint

Implement and register a distinct `GeminiAdapter` only after the contracts
checkpoint pins its exact native version/output profile. The adapter is a
managed distribution target, not a first-party bootstrap target and not a
Gemini extension/marketplace lifecycle adapter.

## Native contract

- Default executable: `gemini`.
- Complete skills: choose portable `~/.agents/skills` globally and
  `<project>/.agents/skills` in projects; also observe `.gemini/skills` as a
  native precedence/unmanaged surface.
- MCP: merge only `mcpServers` in `~/.gemini/settings.json` and
  `<project>/.gemini/settings.json`; preserve unrelated settings and servers.
- Status: bounded `gemini mcp list`, using project root as cwd at project scope.
- Reload: `/mcp reload` is interactive and becomes an actionable next step, not
  a subprocess invocation.
- Trust: project files are declared but not effective until positive workspace
  trust/status evidence exists.

## Implementation boundary

Add `crates/harnesses/src/adapters/gemini.rs` and `gemini_managed.rs`.
`GeminiSkillProjection` supplies roots to the existing standalone project-link
planner. `GeminiManagedProjection` consumes the shared complete source plugin
and owns only Gemini JSON mapping/path decisions. `GeminiEffectiveStateProbe`
owns exact-version status decoding and trust evidence. No extension cache or
native package directory is written.

## Acceptance evidence

- Known exact profile is mutation-authorized in both scopes; unknown versions
  are observe-only.
- Complete skills require no project link because Gemini consumes the canonical
  root directly.
- Stdio/HTTP/SSE definitions map only when semantically faithful; incompatible
  required fields block and optional loss requires acknowledgment.
- Global/project precedence, preservation of unrelated JSON/native skills,
  drift, pending recovery, owned removal, rollback, and immediate-repeat no-op
  pass through shared matrices.
- Trusted MCP status verifies load. Untrusted/unknown status is attention
  required and does not create successful ownership evidence.
- Gemini extension and cache paths remain untouched.
