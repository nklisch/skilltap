# Shared instructions and bridge health

`~/AGENTS.md` is skilltap's canonical global instruction file. It is the
source that should be kept consistent when managing the user's own machine.
Codex and Claude consume different native paths, so a healthy bridge must be
observed rather than assumed:

- Codex normally reads `${CODEX_HOME:-$HOME/.codex}/AGENTS.md`; a non-empty
  `${CODEX_HOME:-$HOME/.codex}/AGENTS.override.md` takes precedence.
- Claude normally reads `~/.claude/CLAUDE.md` and does not read `AGENTS.md`
  directly.
- At project scope Codex layers `AGENTS.md` files from repository root toward
  the working directory. Claude uses `CLAUDE.md` files and can use an import
  or symlink bridge to the project's `AGENTS.md`.

The configured Claude bridge mode is either a symlink to the canonical file or
an import shim containing `@AGENTS.md`. Use `instructions status` to inspect
the effective paths and `instructions setup`/`repair` only after reviewing the
plan. A divergent native file, effective Codex override, broken link, or
unexpected owner is drift. skilltap reports it and preserves user-authored
content; an agent must not replace it silently.

Global and project scopes are independent. A project bridge does not make a
shared repository policy or ask collaborators to install skilltap. Explain
the concrete project path and target harness when reporting a conflict.

Instruction bridges are one part of overall health. Continue to `status` for
enabled/reachable harnesses, native resources, managed skill fingerprints, and
partial previous applications. Use `plan` before any repair that changes a
native file or creates a managed link.
