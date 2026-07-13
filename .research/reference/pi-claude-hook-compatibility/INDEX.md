# Pi / Claude hook compatibility bibliography

Primary sources fetched for the `pi-claude-hook-compatibility` campaign
(facet 2: hook event, payload, ordering, blocking, and failure semantics).
Numbers are append-only within this corpus.

> **Source license:** Mixed upstream documentation and package licenses; consult
> each publisher. Local package and SDK sources are read from the installed
> artifacts on this machine and are version-pinned below.

## Tag vocabulary

`pi`, `pi-hooks`, `claude-code`, `hooks`, `events`, `semantics`, `configuration`,
`lifecycle`, `blocking`, `async`

## Entries

### 1. `@hsingjui/pi-hooks` package source — `pi-hooks-source`

- **Source class:** installed package source (TypeScript)
- **Author:** hsingjui (`@hsingjui/pi-hooks`, MIT)
- **Source path:** `/home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/`
- **Version:** 0.0.2 (read from `package.json`)
- **Original date:** not stated in package; living package
- **Ingested:** 2026-07-12
- **Raw fetch:** source read in place (installed npm package); no test files ship in the package
- **Themes:** pi-hooks, hooks, events, configuration, blocking, async
- **Covers:** Extension entry point, hook config loading/merging, command executor,
  matcher and `if` evaluation, per-event registration that maps Pi events to the
  Claude Code hook surface, JSON output parsing, and tool-result patching.

### 2. Pi extension event reference — `pi-extension-events`

- **Source class:** product / SDK documentation
- **Author:** Pi / Earendil (`@earendil-works/pi-coding-agent`)
- **Source path:** `/home/nathan/.local/share/mise/installs/node/24.17.0/lib/node_modules/@earendil-works/pi-coding-agent/docs/extensions.md`
- **SDK version:** 0.80.6 (read from `package.json`)
- **Original date:** not stated; living documentation
- **Ingested:** 2026-07-12
- **Raw fetch:** none locally (read the installed doc directly)
- **Themes:** pi, events, hooks, lifecycle, blocking
- **Covers:** Authoritative `pi.on(...)` event catalogue — event names, fired-at
  timing, event payload fields, return-value / blocking semantics, and the
  handler `ctx` shape, for every event pi-hooks binds to (`session_start`,
  `session_shutdown`, `session_before_compact`, `session_compact`,
  `before_agent_start`, `agent_end`, `tool_call`, `tool_result`, `input`).

### 3. Claude Code hooks reference — `claude-hooks-reference`

- **Source class:** product reference
- **Author:** Anthropic
- **Source URL:** https://code.claude.com/docs/en/hooks
- **Acquisition:** canonical page fetched via the Mintlify `.md` endpoint
  (`https://code.claude.com/docs/en/hooks.md`); the rendered SPA shell was
  discarded in favour of the markdown body
- **Original date:** not stated; living documentation (inline min-version
  markers up to v2.1.205 observed)
- **Ingested:** 2026-07-12
- **Raw fetch:** none locally
- **Themes:** claude-code, hooks, events, configuration, blocking, async
- **Covers:** Authoritative Claude Code hook contract — full event set, hook
  types (`command` / `http` / `mcp_tool` / `prompt` / `agent`), matcher and `if`
  rules, common and per-event input fields, exit-code semantics, JSON output
  schema, decision control per event, async hooks, timeouts, and platform notes.
