---
id: idea-drop-custom-tap-format
created: 2026-05-19
tags: [refactor, infra]
---

Remove all skilltap-only custom `.json` / `tap.json` shapes and align installs
exclusively to the standard formats published by major providers (Claude Code
plugin manifests, Codex plugin format, Gemini agent format, generic SKILL.md,
MCP server specs, raw agent files, etc.). The thesis: skilltap should not be
in the business of inventing or promoting its own popular plugin/skill format
— it tracks and multi-installs whatever the upstream ecosystems publish.
This is a deep rework: the tap resolver, source adapters, manifest schema,
publish path, and registry conventions all need to be reconsidered around
"adapt to existing formats, never demand a new one." Captured now because
it reframes a large chunk of the architecture and shouldn't be drafted
inline mid-session.
