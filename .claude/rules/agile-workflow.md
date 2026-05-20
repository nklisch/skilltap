---
description: Agile-workflow substrate navigation rules
paths: ['.work/**', 'docs/**']
---

# Agile-Workflow Rules

The project tracks work as markdown items with YAML frontmatter under `.work/`.
This rule file auto-loads when editing anything under `.work/` or `docs/`.
Conventions specific to this project live in `.work/CONVENTIONS.md`.

## Folder structure

```
.work/
  CONVENTIONS.md             # project-specific overrides
  active/
    epics/                   # kind: epic
    features/                # kind: feature
    stories/                 # kind: story
  backlog/                   # ideas captured via /agile-workflow:park
  releases/
    <version>/               # items bound to a shipped or in-flight release
      release-<version>.md
  archive/                   # done items not bound to a tracked release
  bin/
    work-view                # the primary query tool — use it
```

## Kinds

- **epic** — multi-feature capability arc; decomposes into features.
- **feature** — single coherent capability; decomposes into stories.
- **story** — single-stride implementation unit; depends on other stories.
- **release** — version-binding container; aggregates bound items.

## Stages

Features and stories: `drafting → implementing → review → done`.
Releases: `drafting → implementing → released → done`.

## Frontmatter

Every item carries:

```yaml
---
id: <slug>                          # kebab-case, matches filename without .md
kind: epic | feature | story | release
stage: drafting | implementing | review | released | done
tags: [<tag>, ...]                  # taxonomy in .work/CONVENTIONS.md
parent: <parent-id>                 # optional
depends_on: [<id>, ...]             # optional
release_binding: <version>          # optional, present when bound to a release
created: YYYY-MM-DD
---
```

Backlog items use a minimal subset — `id`, `created`, `tags` — until promoted
via `/agile-workflow:scope`.

## Navigation — use `work-view`

```bash
.work/bin/work-view --ready                # items whose deps are satisfied
.work/bin/work-view --stage review         # items awaiting human review
.work/bin/work-view --stage implementing   # in-progress items
.work/bin/work-view --kind feature         # filter by kind
.work/bin/work-view --tag security         # filter by tag
.work/bin/work-view --parent <id>          # all children of an item
.work/bin/work-view --blocking <id>        # what's blocked on <id>
.work/bin/work-view --help                 # full flag set
```

Prefer `work-view` over grep/find on `.work/` — it parses frontmatter and
resolves dependencies.

## Session-start checklist

When starting any session that touches `.work/` or `docs/`:

1. `work-view --stage review` — anything waiting on the user?
2. `work-view --ready` — what's unblocked and ready to pick up?
3. Skim `docs/VISION.md` / `docs/SPEC.md` if the topic is foundational.

## Rolling-foundation rule

Foundation docs in `docs/` describe the system **now** — never add legacy
notes, "previously we did X", or version-stamped narrative. Git history is the
audit trail. When implementation changes foundation-level behavior, roll the
doc forward in the same stride.

## Item lifecycle — slash commands

- `/agile-workflow:park` — capture an idea into `.work/backlog/`
- `/agile-workflow:scope` — promote backlog into active with declared deps
- `/agile-workflow:epic-design` / `:feature-design` / `:refactor-design` /
  `:perf-design` / `:e2e-test-design` — write design INTO an item's body
- `/agile-workflow:implement` (inline) / `:implement-orchestrator` (parallel)
- `/agile-workflow:review` — review an item at `stage:review`
- `/agile-workflow:fix` — single-stride bug fix
- `/agile-workflow:release-deploy` — run gates, ship, advance bindings
- `/agile-workflow:autopilot` — drain the queue end-to-end

## Test integrity

When running, writing, or modifying tests:

- **File real production bugs as backlog items.** When a test failure surfaces
  an actual product bug (not a stale fixture, drifted assertion, or broken
  mock), park it via `/agile-workflow:park` instead of silently fixing inline
  during a test pass. The backlog item is the audit trail.
- **Fix bad tests in-session.** Stale fixtures, drifted assertions, broken
  mocks, and outdated snapshots are test debt, not product bugs.
- **NEVER game a test to make it pass.** A failing test that documents *why*
  it fails — an inline comment naming the bug, a `skip` linked to a backlog id,
  an `xfail` with a reason — is more honest than a green test that lies.

## Capture rules

- New idea, deferred: `/agile-workflow:park`.
- Active scoping: `/agile-workflow:scope`.
- Single-stride bug fix: `/agile-workflow:fix`.
- Don't write items by hand without going through the skills — they handle
  frontmatter, dependency declaration, and the commit cadence.
