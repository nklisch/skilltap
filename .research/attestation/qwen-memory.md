---
source_handle: qwen-memory
fetched: 2026-07-12
source_url: https://qwenlm.github.io/qwen-code-docs/en/users/features/memory/
provenance: source-direct
substrate_confidence: source-direct
---

# Qwen Code instructions

Qwen loads global and project instruction files and explicitly reads an existing `AGENTS.md` for other AI tools. Its own names are `~/.qwen/QWEN.md`, project `QWEN.md`, and `.qwen/QWEN.local.md`; current release notes document the rename from QWEN.md to AGENTS.md for community consistency.

## Key passages

- The instruction table separates global, project, and local-only files.
- The guide says existing `AGENTS.md` files are read without duplication.
