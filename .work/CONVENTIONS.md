# Agile-Workflow Conventions

Project-specific overlay on top of the plugin's defaults. Anything not specified
here uses the plugin defaults from `agile-workflow/ARCHITECTURE.md` and
`agile-workflow/SPEC.md`.

## Release mapping

**Mode: `tag-based`.**

A release is a semver git tag of the form `vMAJOR.MINOR.PATCH` (current line:
`v3.0.x`). The `release_binding` frontmatter field on items references the tag
string without the leading `v` (e.g. `release_binding: 2.2.6`).

Release notes live at `.work/releases/<version>/release-<version>.md`. Items
bound to a release move from `.work/active/` into the release directory when the
release ships.

## Tag taxonomy

| Tag | Meaning |
|---|---|
| `security` | Touches the security model, scan path, trust policy, secrets. |
| `perf` | Performance-driven work — profile-then-optimize, regressions. |
| `refactor` | Pure structural change with no behavior delta. |
| `content` | Documentation, website copy, README, llms.txt regen. |
| `infra` | CI, release scripts, build tooling, dev-env. |
| `testing` | Test infrastructure, coverage, fixtures, harness. |
| `cleanup` | AI cruft / dead code / drift identified by gates. |
| `documentation` | Foundation-doc drift identified by the docs gate. |
| `research` | Grounded research input; routes to agentic-research and never binds to a release. |
| `scan` | Deep-code-scan engagement scaffold; excluded from the ordinary autopilot queue. |
| `correctness` | Deep-code-scan correctness lane. |
| `tests` | Deep-code-scan test-quality lane. |
| `performance` | Deep-code-scan performance lane. |
| `quality` | Deep-code-scan holistic quality lane. |
| `structure` | Deep-code-scan structure and refactor lane. |
| `architecture` | Deep-code-scan architecture lane. |
| `custom` | Deep-code-scan custom lane. |
| `leaf` | Deep-code-scan leaf altitude band. |
| `module` | Deep-code-scan module altitude band. |
| `subsystem` | Deep-code-scan subsystem altitude band. |
| `system` | Deep-code-scan system altitude band. |

Items can carry multiple tags. Gates emit their `gate_origin:<name>` as an
additional non-taxonomy tag.

## Slug conventions

- Kebab-case, all lowercase, ASCII only.
- Epic: `epic-<topic>` (e.g. `epic-windows-support`).
- Feature: `feature-<topic>` for top-level, or `feature-<parent-topic>-<child>`
  when the parent context matters.
- Story: `story-<topic>` for top-level, or `story-<parent-topic>-<child>` when
  child of a feature.
- Backlog idea: `idea-<topic>` (set by `/agile-workflow:park`).
- Release notes file: `release-<version>.md`.

Child items SHOULD prefix their parent's topic when ambiguity would otherwise
arise; they MAY use a bare topic when the slug is already unique in `.work/`.

## Stage overrides

None. Use the plugin defaults: `drafting → implementing → review → done` for
features and stories; `drafting → implementing → released → done` for releases.

## Research engagements

Research items carry `research_refs` and `research_origin`. Their
`research_dials:` block commissions `scope_authority`, `verification_rigor`,
`intent`, and `output_kind` for the research orchestrator.

## Scan engagements

Scan campaigns and their remediation items carry `scan_origin`. Scan scaffold
items use the `scan` tag plus a lane tag and, for stories, an altitude-band tag.

## Terminal-tier retention

delete-refs

## Gate configuration

`gates_for_release: [security, tests, cruft, docs, patterns]`

```yaml
gate_finding_routing:
  critical: implementing
  high: implementing
  medium: drafting
  low: backlog
  info: skip
gate_refactor_scan_library_roots:
  - .agents/skills
  - .claude/skills
binding_guard: warn
epic_cohesion: phased
backlog_staleness_days: 90
```

All five gates run in the listed order during `/agile-workflow:release-deploy`.
Each gate emits items (not pass/fail reports) tagged `gate_origin:<gate>`.
