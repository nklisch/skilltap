---
description: Manage AGENTS.md as shared Codex and Claude instructions.
---

# Shared Instructions

`~/AGENTS.md` is the canonical global instruction file. skilltap bridges native
harness locations to it so Codex and Claude Code share one maintained source.

```bash
skilltap instructions status
skilltap instructions setup
skilltap instructions repair
```

By default, Claude's `CLAUDE.md` is a symlink to the canonical `AGENTS.md`.
Configuration may instead select a managed import file. Existing independent
instruction files are conflicts until their contents are reconciled; skilltap
does not silently choose one.

For a project, `AGENTS.md` at the resolved project root is canonical. A
`CLAUDE.md` bridge is created only when Claude needs it. If both files contain
independent content, status reports the conflict and repair requires explicit
approval before replacing the Claude file.

Scope follows the common contract:

```bash
skilltap instructions status              # global
skilltap instructions status --project    # current project
skilltap instructions setup --project ~/src/example
```
