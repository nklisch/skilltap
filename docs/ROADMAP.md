# Roadmap

The current state of skilltap, plus what hasn't been scheduled.

## Current state

skilltap is at v2.2.x — typed `install <type> <source>` family, flat `[security]` + `[scanner]` config, `state.json` as the single state store, `skilltap.toml` + `skilltap.lock` with skills/plugins/mcps tables, smart-scope default, TUI dashboard, plugin capture, Claude Code plugin adoption.

There is no in-flight phase. The next release line will start a new entry in this file.

## What's Deferred (no scheduled version)

Real future-work items, not blocked on technical issues — they're either large efforts, platform-specific features, or design problems that haven't been prioritized.

- Windows support
- Linux distro packages (.deb, .rpm, AUR, Nix)
- `security.require_provenance` config option (block unverified skills)
- Direct LLM API integrations for semantic scan (Anthropic API, OpenAI API — bypassing CLI)
- Plugin for popular editors (VS Code extension)
- Skill dependency system
- SBOM generation for installed skills
- Plugin hooks support (Claude Code hooks.json)
- Plugin LSP server support (Claude Code .lsp.json)
- Plugin commands support (Claude Code commands/*.md)
- Agent definitions for non-Claude-Code platforms (when other agents adopt the format)
- Plugin user config / secrets management (Claude Code userConfig with keychain)
