---
id: epic-expanded-harness-support
kind: epic
stage: implementing
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

## Design decisions

- **Does expanded target support broaden first-party plugin bootstrap?** No.
  The self-hosted plugin and bootstrap remain Codex/Claude distribution
  surfaces. Other harnesses participate in ordinary detection, enablement,
  adoption, planning, synchronization, and update flows.
- **Do instructions become an admission requirement?** No. AGENTS.md or native
  instruction support is capability-detected per adapter. Whole-directory
  skills and global/project MCP remain the minimum target contract.
- **Where does shared acceptance infrastructure live?** With the typed registry
  and adapter contract, then inside each adapter feature's delivery evidence;
  there is no test-only feature or parallel adapter framework.
- **How are Pi companions owned?** Existing MCP and Claude-hook extensions stay
  user-owned unless the user explicitly installs or adopts them through a
  supported future lifecycle. Detection alone never transfers ownership.
- **How is Pi's hook prerequisite grounded?** A dedicated research engagement
  must attest the exact extension, version/health contract, and hook semantics
  before the Pi adapter can be designed or granted mutation authority.

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

## Decomposition

The existing managed-fallback feature remains the shared publication
foundation. Eight additional features split the work by capability: one
registry and adapter contract, four independent direct-adapter families, a Pi
contract research prerequisite, the Pi compound adapter, and independent
candidate admission. Nine children exceed the usual epic target because this
scope spans fifteen harnesses; collapsing them further would combine unrelated
native contracts or hand oversized features to the next design pass.

### Child features

- `epic-expanded-harness-support-registry` — typed target registry,
  configuration/composition, and reusable adapter acceptance contract — depends
  on: `[]`.
- `feature-managed-fallback-target-parity` — shared complete-skill and MCP
  projection lifecycle for targets without native distribution — depends on:
  `[epic-cross-harness-materialization,
  epic-expanded-harness-support-registry]`.
- `epic-expanded-harness-support-file-managed` — Gemini, OpenCode, and Kiro
  adapters — depends on: `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`.
- `epic-expanded-harness-support-native-coexistence` — Factory Droid, Qwen,
  and Copilot adapters with native-managed coexistence — depends on:
  `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`.
- `epic-expanded-harness-support-configuration-constrained` — Kimi, Vibe, and
  Kilo adapters with explicit reload, transport, and document constraints —
  depends on: `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`.
- `epic-expanded-harness-support-trust-interactive` — Junie and Amp adapters
  with declared-versus-effective trust and interactive-state behavior —
  depends on: `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`.
- `epic-expanded-harness-support-pi-hook-research` — attest the exact Pi Claude
  hook-compatibility extension and its health/version/semantics contract —
  depends on: `[]`.
- `epic-expanded-harness-support-pi` — conditional Pi compound adapter —
  depends on: `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity,
  epic-expanded-harness-support-pi-hook-research]`.
- `epic-expanded-harness-support-candidate-admission` — independently validate
  and admit Cursor, Zoo Code, and ZCode — depends on:
  `[epic-expanded-harness-support-registry,
  feature-managed-fallback-target-parity]`.

### Simplification arcs

- `epic-expanded-harness-support-registry` removes repeated target lists and
  dispatch matches from configuration, CLI, composition, status, and fixtures.
- `feature-managed-fallback-target-parity` consolidates managed acquisition,
  projection, ownership, drift, update, removal, and verification instead of
  duplicating them in each adapter.
- Concrete adapter features reuse bounded execution, target-local state,
  rollback, and effective-load verification while retaining only target-owned
  codecs, probes, paths, and lifecycle semantics.

### Decomposition risks

- Exact target versions, write paths, reload behavior, and trust constraints
  may move before implementation. No adapter gains mutation authority without
  refreshed source evidence and isolated native validation.
- Pi's required Claude-hook compatibility extension is not yet attested in the
  research substrate. The dedicated research child blocks Pi design until it
  verifies the exact extension and semantics.
- Cursor, Zoo Code, and ZCode have different missing boundaries. Their gates
  are target-local; partial success cannot produce batch support claims.
- Registry generalization must preserve the intentionally narrower
  Codex/Claude self-hosted plugin and bootstrap contract.

## Other agent review

- Invoked because: large architectural expansion across registry, adapter,
  capability, and native-contract boundaries.
- Phase 1 — advisory/completeness: same-harness fresh-context review found the
  missing Pi research prerequisite, oversized/heterogeneous feature groups,
  weakened two-scope wording, and stale project-agent guidance.
- Phase 2 — adversarial verification: the same reviewer re-read the corrected
  decomposition and returned `ready` with no remaining material findings.
- Fixed/active blockers: added the Pi hook research dependency; separated Pi
  delivery from candidate admission; split constrained targets; required both
  scopes; rolled `AGENTS.md` forward.
- Parked: none.
- Rejected: none.
- Skipped/degraded: different-model review was unavailable, so both passes are
  labeled same-harness fresh-context rather than cross-model.
