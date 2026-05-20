# Migration Report — agile-workflow bootstrap

Date: 2026-05-19
Source shape detected: **workflow-plugin**
Mode: **bootstrap** (auto, no prior `.work/` substrate)

## Foundation docs detected (preserved)

- `docs/VISION.md`
- `docs/SPEC.md`
- `docs/ARCH.md`
- `docs/UX.md`
- `docs/SECURITY.md`
- `docs/ROADMAP.md`

All foundation docs are left in place. They remain the source of truth for
"what skilltap is today." Convert never edits these.

## Items seeded

### `.work/active/features/`

- `feature-cleanup-current-state.md` — stage: `review`, tags: `[cleanup, content, refactor]`
  - Source: `docs/designs/cleanup-current-state.md`
  - Rationale: most code-level units (plugin-v2 → skilltap-plugin rename,
    e2e-v2 → e2e rename, InstalledJson schema drop) have already landed in
    `main`. `docs/designs/completed/` still holds one residual file
    (refactor-project-wide-cleanup.md), so Unit 24 is partially outstanding.
    Marked `review` so a human can close it out.

### `.work/releases/v0/`

- `release-v0.md` — stage: `released`, synthetic container for pre-substrate history.
- `feature-refactor-project-wide-cleanup.md` — stage: `done`, `release_binding: "0"`
  - Source: `docs/designs/completed/refactor-project-wide-cleanup.md`
  - Rationale: explicitly marked complete in commit `1ec2f6b`.

### `.work/backlog/`

- `idea-universal-agents-md-conversion.md` — captured user idea (universal
  `AGENTS.md` / `CLAUDE.md` / `GEMINI.md` conversion command with symlinks).
- `idea-drop-custom-tap-format.md` — captured user idea (drop skilltap-only
  `.json` tap formats; align installs to upstream provider standards).

## Files left in place (legacy history)

- `docs/designs/cleanup-current-state.md` — source for the seeded feature item.
  Once the feature reaches `stage: done`, this file can be deleted (the body
  lives in the substrate now).
- `docs/designs/completed/refactor-project-wide-cleanup.md` — source for the
  seeded v0 release item. Same disposal pattern.
- `docs/ROADMAP.md` — already trimmed to "Current state" + "Deferred list"
  framing per the cleanup design. The roadmap continues to be the
  forward-looking list of unscheduled work; no epic decomposition was forced
  during bootstrap.

## Conventions chosen

| Setting | Value |
|---|---|
| Release mapping | `tag-based` (semver tags, current line `v2.2.x`) |
| Tag taxonomy | `security, perf, refactor, content, infra, testing, cleanup, documentation` |
| Slug convention | kebab-case; child items optionally prefix the parent's topic |
| Stage overrides | none (plugin defaults) |
| Gates for release | `[security, tests, cruft, docs, patterns]` (default order) |

Full conventions in `.work/CONVENTIONS.md`. Edit there if anything should
change — the file is user-owned and `convert --update` never touches it.

## Artifacts installed

- `.work/CONVENTIONS.md`
- `.work/bin/work-view` (executable)
- `.claude/rules/agile-workflow.md`
- `AGENTS.md` — appended agile-workflow section between
  `<!-- agile-workflow:start -->` / `<!-- agile-workflow:end -->` markers.
  (`.claude/CLAUDE.md` is a symlink to `AGENTS.md`, so the section flows
  through to Claude Code's CLAUDE.md surface automatically.)

## Next steps

- `work-view --stage review` — close out `feature-cleanup-current-state`
  once you confirm Units 24 (`docs/designs/completed/` deletion) and 25
  (`llms-full.txt` regen) are done.
- `/agile-workflow:scope` — promote the two parked backlog items into
  active when you're ready to scope them.
- `/agile-workflow:epicize` — optional; seed any new epics directly from
  `docs/VISION.md` if you want richer planning structure.

## Rollback

Single-commit migration. `git revert HEAD` cleanly restores the pre-bootstrap
state. All source files (designs, docs, roadmap, CLAUDE/AGENTS content) were
preserved.
