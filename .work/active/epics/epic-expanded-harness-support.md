---
id: epic-expanded-harness-support
kind: epic
stage: drafting
tags: []
parent: null
depends_on: [epic-cross-harness-materialization, epic-harness-observation-adoption, epic-reconciliation-execution]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Expanded Harness Support

## Brief

Extend skilltap's deeply supported target set beyond Codex and Claude Code to
every currently qualified harness that can load complete skill directories and
MCP configuration through documented global and project surfaces. Native
marketplace and plugin lifecycles remain preferred when available; otherwise
skilltap owns acquisition, managed projection, revision tracking, drift,
update, and removal without writing undocumented caches.

The direct target set is Factory Droid, Qwen Code, GitHub Copilot CLI, Gemini
CLI, Junie, Kimi Code CLI, OpenCode, Kilo Code, Mistral Vibe, Kiro CLI, and Amp.
Cursor, Zoo Code, and ZCode are included as boundary-validation tracks and do
not gain mutation support until their exact supported write paths are verified.
Pi is a conditional compound target: mutation support is enabled only when a
compatible MCP extension and Claude Code hook-compatibility extension are
installed and healthy for that user.

Support means the same thing for every admitted target: explicit detection,
version-bounded mutation authority, global and project scopes, whole-directory
skills, faithful MCP translation, effective-state observation, managed
ownership, safe update/removal, idempotent reconciliation, and exact partial or
blocked reporting for unsupported optional or required components.

## Strategic decisions

- **Which harnesses are in scope?** All eleven research-qualified direct
  targets, Pi as a conditional compound target, and the three boundary-spike
  candidates. A candidate becomes supported only after it clears the same
  adapter acceptance contract.
- **Does a target need a marketplace or plugin manager?** No. Documented,
  observable global and project skill and MCP surfaces are the admission bar.
  Native lifecycle remains preferred when it exists.
- **How is Pi admitted?** Treat Pi plus the user's installed MCP and Claude Code
  hook-compatibility extensions as one capability profile. Detect and report
  companion-extension health separately; never pretend Pi core supplies those
  capabilities.
- **Are currently excluded harnesses included?** No. Goose, Windsurf/Devin
  Desktop Cascade, and Cline remain excluded until they document an ambient
  project-scoped MCP surface. Roo Code remains excluded because it is retired.
- **What does `--target all` mean?** Every enabled harness in the typed target
  registry, not a hard-coded Codex/Claude pair.

## Simplification opportunity

Replace repeated Codex/Claude target enumerations in CLI parsing, configuration,
rendering, composition, fixtures, and application dispatch with one typed target
registry. Retain genuinely harness-specific adapters and native contracts; do
not flatten their schemas or lifecycle behavior into a universal plugin format.

## Foundation references

- `docs/VISION.md` — broad target direction with deep-support admission rules.
- `docs/SPEC.md` — extensible harness identifiers, target selection, and
  conditional-target semantics.
- `docs/ARCH.md` — registry-driven adapter composition and distinct native
  boundaries.
- `docs/UX.md` — target flags enumerate enabled registered harnesses.
- `docs/HARNESS-CONTRACTS.md` — expansion set, boundary gates, and Pi compound
  capability profile.

## Acceptance direction

- Every direct target passes the shared isolated adapter acceptance matrix for
  detection, both scopes, complete skills, MCP, observation, reload, drift,
  removal, and immediate-repeat idempotency.
- Native plugin distributions remain independently tracked and preferred over
  managed fallback whenever the same plugin exists for a target.
- Unknown target versions remain observe-only; runtime probes may narrow but
  never grant mutation authority.
- Cursor, Zoo Code, and ZCode stay observe-only candidates until their boundary
  spikes attest exact supported files and reload behavior.
- Pi status distinguishes the core harness, MCP extension, and Claude-hook
  extension. Missing or incompatible companions keep the compound adapter
  observe-only and produce actionable health output.
- Optional unsupported components require foreground acknowledgment; required
  unsupported components remain blocked even with `--yes`.

## Anticipated decomposition

- Typed target registry, configuration schema, CLI parsing, and composition.
- Shared managed-adapter ports and reusable skill/MCP projection machinery.
- Direct adapter waves following the research-recommended boundary order.
- Pi compound capability detection, ownership, and lifecycle integration.
- Cursor, Zoo Code, and ZCode isolated boundary spikes and admission gates.
- Cross-target fixtures, native validation, compatibility, status, help, and
  website/documentation updates.
