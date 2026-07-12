---
id: epic-skilltap-plugin-distribution-guidance
kind: feature
stage: done
tags: [content]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package, epic-skilltap-plugin-distribution-cli-contract, epic-skilltap-plugin-distribution-bootstrap]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Skilltap Agent Guidance

## Brief

Author the single portable `skilltap` skill carried by the plugin. It should
help an agent recognize when to invoke the binary, choose among status,
adopt, plan, sync, lifecycle, instruction, and daemon commands, understand
global/project scope, locate configuration and managed state, diagnose common
health results, and explain the next user decision. It should link agents to
the executable help surface for exact syntax rather than freezing a duplicate
command reference in prose.

The skill remains harness-neutral and contains the complete directory
artifact, with supporting references only when they materially improve
diagnosis. This feature also defines the replacement/deprecation wording for
the obsolete skilltap-adjacent guidance, while the actual sibling repository
archive is handled by the final cutover feature.

The skill is implicitly available to agents. It points agents to the
self-bootstrap flow, direct `--help`, status/plan/sync diagnostics, and the
latest-compatible update policy without turning the skill into a second CLI
implementation.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: consumer of the package, CLI, and bootstrap contracts;
  release integration packages the resulting skill.

## Foundation references

- `docs/VISION.md` — Agent Forward, Non-Goals
- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Output, Configuration
  Directory, Status, Planning, Synchronization
- `docs/UX.md` — Help and Diagnostic Discovery, Target and Scope
- `README.md` and `website/guide/` — current user-facing operating model
- `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

## Design decisions

- **Invocation policy**: The skill is implicitly available to agents so they
  can recognize self-setup, status, and recovery requests without a manual
  invocation ceremony.
- **Guidance boundary**: The skill explains bootstrap, harness detection,
  configuration layout, update policy, and diagnostic next actions at a high
  level; direct `--help` remains authoritative for exact syntax.

<!-- Feature design will define the skill's progressive-disclosure sections and
reference boundaries. No UI mockups apply to this terminal/skill surface. -->

## Architectural choice

Use one concise portable `SKILL.md` as the activation surface and two
progressively loaded Markdown references for configuration and diagnostics.
The skill body gives an agent the purpose, decision tree, scope/target model,
bootstrap entry point, and safety boundary; references carry the less-frequently
needed state-layout and recovery detail. Exact flags and output schemas remain
the compiled binary's `--help` contract. This keeps the skill useful in both
harnesses without freezing a second CLI manual.

Alternative approaches considered:

1. Put every command, flag, and output field in `SKILL.md`. This is easy to
   discover but duplicates and quickly drifts from the executable contract.
2. Ship only a one-line description. This avoids drift but leaves agents
   unable to choose a safe first command or diagnose configuration health.
3. Chosen: a short decision-oriented body with references loaded only when an
   agent needs state or recovery detail.

The highest-risk unit is the activation body: its description and first
paragraph must trigger for setup, health, reconciliation, and troubleshooting
requests while never inviting marketplace discovery or recommendations.

## Implementation Units

### Unit 1: Portable activation and command-routing skill

**File**: `plugin/skills/skilltap/SKILL.md`
**Story**: `story-skilltap-plugin-distribution-guidance-core`

The complete skill directory remains the managed unit. The frontmatter uses
only portable `name` and `description`; the body introduces the binary,
bootstrap/status/adopt/plan/sync/lifecycle/instructions/daemon command families,
global versus project scope, target selection, `--json`, `--yes`, and the rule
to ask the executable for exact syntax. It explicitly says skilltap never
searches or recommends marketplace contents and that unsupported or partial
operations must be surfaced to the user.

**Acceptance criteria**:

- [ ] Frontmatter conforms to the Agent Skills required fields and the parent
      directory/name identity rule.
- [ ] An agent can select a first command for setup, health, reconciliation,
      lifecycle, instruction, update, or daemon requests without a copied flag
      table.
- [ ] The body names `skilltap --help` and leaf help as authoritative and does
      not describe marketplace search, ranking, or recommendations.
- [ ] The body preserves the separate binary and harness setup results and
      `--yes`/partial consequence boundary.

### Unit 2: Configuration and instruction-layout reference

**Files**:

- `plugin/skills/skilltap/references/configuration.md`
- `plugin/skills/skilltap/references/instructions.md`

**Story**: `story-skilltap-plugin-distribution-guidance-layout`

References explain the machine-wide XDG configuration directory, the roles of
`config.toml`, `inventory.toml`, `state.json`, and `managed/`, plus global and
project scope. The instruction reference explains `~/AGENTS.md` as canonical,
Codex/Claude bridge paths, precedence/drift warnings, and that the complete
skill directory (not only `SKILL.md`) is managed. References are linked from
the body and use current foundation terminology without adding a new config
schema.

**Acceptance criteria**:

- [ ] Every path and ownership claim matches `docs/SPEC.md` and `docs/ARCH.md`.
- [ ] The references distinguish desired inventory, machine-written state,
      native declared state, and effective installed state.
- [ ] Global defaults, `--project`, `--project <path>`, and `--all-scopes` are
      explained without implying that project metadata is shared with
      collaborators.
- [ ] AGENTS/CLAUDE bridge precedence and divergence are framed as diagnostics,
      not silent overwrite instructions.

### Unit 3: Diagnostic and update/recovery reference

**File**: `plugin/skills/skilltap/references/diagnostics.md`
**Story**: `story-skilltap-plugin-distribution-guidance-diagnostics`

This reference maps status/plan/sync outcomes, result classes, warnings,
attention, partial consequences, next actions, binary bootstrap outcomes,
Git-SHA updates, and optional daemon behavior to an agent-to-user explanation
workflow. It tells the agent when to stop, summarize the consequence, and ask
the user for a decision; it never invents a bypass or generic confirmation.

**Acceptance criteria**:

- [ ] Healthy, changes-needed, attention, partial, blocked, and unavailable
      outcomes each have a concise user-facing interpretation and next step.
- [ ] Binary update policy distinguishes same-major safe updates, opt-out, and
      explicit major-version acknowledgment; plugin/harness setup remains
      separately reported.
- [ ] Daemon guidance says it never acknowledges partial work, overwrites drift,
      or replaces foreground confirmation.
- [ ] JSON is described as a stable representation of the same semantics as
      plain output, with `--help` used for exact fields.

### Unit 4: Guidance artifact validation

**Files**:

- `crates/cli/tests/plugin_package.rs` (or a focused plugin-guidance test)
- `plugin/skills/skilltap/SKILL.md` and `plugin/skills/skilltap/references/*`

**Story**: `story-skilltap-plugin-distribution-guidance-validation`

Extend package validation to load the whole skill directory, require the
portable frontmatter contract, verify reference links stay inside the skill,
and reject stale discovery/search language or a duplicated command grammar.
Tests remain offline and fixture-based; they do not execute a harness or
network request.

**Acceptance criteria**:

- [ ] Package validation fails when `SKILL.md` is missing, renamed, malformed,
      or detached from its complete sibling resources.
- [ ] Every linked reference exists beneath the skill root and no path escapes
      that root.
- [ ] The published plugin package test proves the guidance is present in both
      native channel trees without duplicating the skill directory.
- [ ] A repeatable validation pass reports no discovery/recommendation claims
      and no stale hard-coded leaf flag table.

## Implementation Order

1. `story-skilltap-plugin-distribution-guidance-core`
2. `story-skilltap-plugin-distribution-guidance-layout` and
   `story-skilltap-plugin-distribution-guidance-diagnostics` (parallel after
   the core body establishes links and terminology)
3. `story-skilltap-plugin-distribution-guidance-validation` (after all prose
   files exist)

## Testing

- Unit/package tests parse frontmatter and validate strict skill-directory
  boundaries, reference links, and no-search language.
- Offline integration fixtures load the package from the repository and assert
  both Claude and Codex manifests point at one complete skill tree.
- Documentation review checks every command/path/version claim against the
  foundation docs and compiled `--help` output; the skill does not become a
  second command-schema source.

## Risks

- The main risk is guidance drift as CLI help evolves. Keep exact syntax out of
  prose and make validation assert only durable concepts and links.
- A broad description can accidentally trigger marketplace discovery behavior.
  Use explicit activation conditions around managing the caller's local
  environment and repeat the no-search boundary in the body.
- References can become an unbounded manual. Limit them to configuration,
  instruction bridges, diagnostics, and update/recovery decisions; route all
  other questions to executable help.

## Design decisions

- **Progressive disclosure**: one body plus configuration/instruction and
  diagnostic/update references — enough orientation without a duplicate CLI
  manual.
- **Skill boundary**: all references remain siblings inside the complete
  `plugin/skills/skilltap/` directory and are shipped as one artifact.
- **Validation ownership**: package validation checks structure and links;
  release validation owns version/source parity and website/install alignment.

## Children complete

The four guidance stories are complete: portable activation body, configuration
and instruction references, diagnostic/update/recovery reference, and offline
artifact validation.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Feature acceptance review at standard weight. The complete skill
directory provides a concise portable activation body plus progressive
configuration/instruction and diagnostic references; package validation proves
the reference tree and rejects unsafe/discovery or duplicate-command drift.
All four child stories are independently approved and the offline plugin
package suite passes.
