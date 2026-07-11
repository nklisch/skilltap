---
source_handle: codex-config
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/config-file/config-basic
provenance: source-direct
substrate_confidence: source-direct
---

# Config basics

## Summary

OpenAI documents a layered TOML configuration model shared by Codex CLI and
the IDE extension. User configuration lives at `~/.codex/config.toml` and
trusted projects may add `.codex/config.toml` layers from repository root to
working directory. Explicit command-line overrides have highest precedence;
system and built-in defaults have lowest precedence.

## Key passages

### Files and trust (lines 699-709)

> "Your personal defaults live in `~/.codex/config.toml`." (Codex configuration
> file, line 699)

- User defaults live at `~/.codex/config.toml`.
- Project and subfolder overrides use `.codex/config.toml`.
- Codex loads project `.codex` layers only for trusted projects.
- CLI and IDE share the same configuration layers.

### Precedence (lines 710-721)

- Precedence from highest to lowest is CLI flags and `--config`; project
  `.codex/config.toml` files from root to current directory with closest
  winning; selected profile; user config; system config; built-in defaults.
- Untrusted projects lose all project-scoped `.codex` layers, including
  project config, hooks, and rules, while user and system configuration remain.
