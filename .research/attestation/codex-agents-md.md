---
source_handle: codex-agents-md
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/agent-configuration/agents-md
provenance: source-direct
substrate_confidence: source-direct
---

# Custom instructions with AGENTS.md

## Summary

OpenAI documents layered Codex instructions with a global file in Codex home
and project files from the repository root down to the working directory.
Override files take precedence within each directory. Project layers are
concatenated root-to-leaf subject to a configurable byte limit, so closer
instructions occur later and override earlier guidance.

## Key passages

### Discovery and merge order (lines 699-706)

> "Codex reads `AGENTS.md` files before doing any work." (Instructions discovery,
> line 699)

- Global scope is the Codex home directory, defaulting to `~/.codex` unless
  `CODEX_HOME` is set.
- At global scope, Codex uses the first non-empty file among
  `AGENTS.override.md` and `AGENTS.md` in that priority order.
- Project discovery starts at the project root and walks down to the current
  working directory. In each directory it tries `AGENTS.override.md`, then
  `AGENTS.md`, then configured fallback filenames, taking at most one file.
- Files concatenate from project root toward the working directory, so closer
  instructions appear later.
- Empty files are skipped and collection stops at `project_doc_max_bytes`,
  whose documented default is 32 KiB.

### Global and project usage (lines 707-788)

- The documented global file is `~/.codex/AGENTS.md`, with
  `~/.codex/AGENTS.override.md` available as a temporary replacement.
- Repository-root `AGENTS.md` adds project norms while inheriting global
  guidance; nested overrides can specialize subdirectories.
- Discovery stops at the current directory.
- `project_doc_fallback_filenames` adds alternate project instruction names;
  `project_doc_max_bytes` changes the combined size limit.
