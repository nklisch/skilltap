---
id: idea-universal-agents-md-conversion
created: 2026-05-19
tags: []
---

A universal "agents config" conversion command that, given a project root,
creates a canonical `AGENTS.md` and sets up symlinks/aliases at both the
project level (`.claude/CLAUDE.md`, `.gemini/GEMINI.md`, etc.) and the global
user level — so a single source of truth feeds every coding agent. Should
work whether the project starts from `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, or
nothing, and should be a fast one-shot command. Could ship as a skilltap
subcommand (`skilltap unify-agents-md` or similar) or as a small standalone
companion CLI.
