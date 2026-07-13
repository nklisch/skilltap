---
source_handle: claude-hooks-reference
fetched: 2026-07-12
source_url: https://code.claude.com/docs/en/hooks.md
provenance: source-direct
substrate_confidence: source-direct
acquisition: canonical page fetched via Mintlify markdown endpoint (https://code.claude.com/docs/en/hooks.md)
---

# Claude Code hooks reference

## Summary

Anthropic's "Hooks reference" (canonical URL `https://code.claude.com/docs/en/hooks`,
also reachable from `docs.claude.com` and `docs.anthropic.com` redirects)
documents the authoritative Claude Code hook contract. Hooks are "user-defined
shell commands, HTTP endpoints, or LLM prompts that execute automatically at
specific points in Claude Code's lifecycle." Five hook types are defined
(`command`, `http`, `mcp_tool`, `prompt`, `agent`), with a per-event type-support
matrix. The event set is large (~30 events), far broader than the nine pi-hooks
exposes. Command hooks receive JSON on stdin and communicate through exit codes,
stdout JSON, and stderr; the JSON output schema and per-event decision control
are versioned (inline min-version markers up to v2.1.205 appear).

## Anchored excerpts and source-internal facts

### Hook events (full set)

- Per-session: `SessionStart`, `SessionEnd`. Per-turn: `UserPromptSubmit`,
- `Stop`, `StopFailure`. Per-tool-call inside the agentic loop: `PreToolUse`,
  `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `PermissionRequest`,
  `PermissionDenied`. Subagent / team: `SubagentStart`, `SubagentStop`,
  `TaskCreated`, `TaskCompleted`, `TeammateIdle`. Compaction: `PreCompact`,
  `PostCompact`. Setup / instructions: `Setup`, `InstructionsLoaded`. Display /
  notification: `Notification`, `MessageDisplay`. Other: `UserPromptExpansion`,
  `ConfigChange`, `CwdChanged`, `FileChanged`, `WorktreeCreate`,
  `WorktreeRemove`, `Elicitation`, `ElicitationResult`. (Hook lifecycle diagram
  and event table.)

### Hook types and per-event support

- Five types: `command`, `http`, `mcp_tool`, `prompt`, `agent`.
- Events supporting all five: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`,
  `PostToolBatch`, `PermissionRequest`, `PermissionDenied`, `Stop`,
  `SubagentStop`, `TaskCreated`, `TaskCompleted`, `TeammateIdle`,
  `UserPromptSubmit`, `UserPromptExpansion`.
- Events supporting `command`/`http`/`mcp_tool` only: `ConfigChange`,
  `CwdChanged`, `Elicitation`, `ElicitationResult`, `FileChanged`,
  `InstructionsLoaded`, `Notification`, `PostCompact`, `PreCompact`,
  `SessionEnd`, `StopFailure`, `SubagentStart`, `WorktreeCreate`,
  `WorktreeRemove`.
- `SessionStart` and `Setup` support `command` and `mcp_tool` only (no `http`,
  `prompt`, or `agent`).

### Common input fields

- `session_id`, `prompt_id` (v2.1.196+; absent until first user input),
  `transcript_path` (written async; may lag the in-memory conversation),
  `cwd`, `permission_mode` (`"default"`/`"plan"`/`"acceptEdits"`/`"auto"`/
  `"dontAsk"`/`"bypassPermissions"`; not all events carry it), `effort`
  (`{level: "low"|"medium"|"high"|"xhigh"|"max"}` for tool-context events), and
  `hook_event_name`.
- In `--agent` / subagent context, additional fields `agent_id` and `agent_type`
  are included.
- `model` is supplied **only** to `SessionStart` hooks, and even then "not
  guaranteed to be present." There is no `$CLAUDE_MODEL` env var.

### Matcher and `if` rules

- `matcher` is a single regex for most events; alternatives separated by `|` or
  `,` (every matcher-supporting event except `FileChanged`/`StopFailure`, which
  use a narrower exact-match set of letters/digits/`_`/`|`).
- `if` is a permission-rule syntax (e.g. `"Bash(git *)"`, `"Edit(*.ts)"`)
  evaluated **only on tool events** (`PreToolUse`, `PostToolUse`,
  `PostToolUseFailure`, `PermissionRequest`, `PermissionDenied`); "On other
  events, a hook with `if` set never runs." Uses the same syntax as permission
  rules.
- Events with **no matcher support** (always fire, matcher silently ignored):
  `UserPromptSubmit`, `PostToolBatch`, `Stop`, `TeammateIdle`, `TaskCreated`,
  `TaskCompleted`, `WorktreeCreate`, `WorktreeRemove`, `MessageDisplay`,
  `CwdChanged`.
- PreToolUse tool-name matcher values are the **capitalised** tool names: `Bash`,
  `Edit`, `Write`, `Read`, `Glob`, `Grep`, `Agent`, `WebFetch`, `WebSearch`,
  `AskUserQuestion`, `ExitPlanMode`, plus `mcp__.*`.

### Hook handler fields and timeouts

- Handler fields: `type` (required), `command`/`prompt`/etc., `matcher`,
  `if`, `timeout` (seconds), `async` (command only), `args` (exec form),
  `shell: "powershell"` (Windows).
- Default `timeout`: **600** for `command`/`http`/`mcp_tool`; **30** for
  `prompt`; **60** for `agent`. `UserPromptSubmit` lowers the
  `command`/`http`/`mcp_tool` default to **30**; `MessageDisplay` lowers it to
  **10**; `SessionEnd` has a default of **1.5 seconds** (with a budget raised to
  the highest per-hook timeout up to 60s, overridable via
  `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS`).
- `${CLAUDE_PROJECT_DIR}`, `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`
  placeholders are rewritten. `CLAUDE_ENV_FILE` is available to
  `SessionStart`/`Setup`/`CwdChanged`/`FileChanged` for env-var persistence.

### Exit codes

- Exit 0: success; stdout parsed for JSON. **JSON is processed only on exit 0.**
  For most events stdout is debug-logged but not transcripted; the exceptions are
  `UserPromptSubmit`, `UserPromptExpansion`, and `SessionStart`, where stdout is
  added as context Claude can see.
- Exit 2: blocking error; stderr fed back to Claude. Effect is per-event (table):
  `PreToolUse` blocks the tool call; `UserPromptSubmit` rejects the prompt;
  `Stop`/`SubagentStop` prevent stopping; `PreCompact` blocks compaction;
  `PostToolBatch`/`TaskCreated`/`TaskCompleted` roll back; etc. `PostToolUse` /
  `PostToolUseFailure` exit 2 shows stderr to Claude (the tool already ran /
  already failed). `SessionStart`/`SessionEnd`/`Notification`/`Setup` exit 2
  shows stderr to user only.
- Any other exit code: non-blocking error for most events; a `<hook name> hook
  error` notice with the first stderr line is transcripted. `WorktreeCreate` is
  the exception (any non-zero aborts).

### JSON output schema

- Universal fields: `continue` (false stops Claude entirely, precedence over
  event decisions), `stopReason`, `suppressOutput`, `systemMessage`,
  `terminalSequence` (v2.1.141+; allowlisted OSC 0/1/2/9/99/777 and BEL only).
- "You must choose one approach per hook, not both: either use exit codes alone
  for signaling, or exit 0 and print JSON." JSON is ignored on exit 2.
- Output strings (`additionalContext`, `systemMessage`, plain stdout) are
  **capped at 10,000 characters**; overflow is saved to a file and replaced with
  a preview + path.
- `additionalContext` is returned inside `hookSpecificOutput` with a
  `hookEventName`; it is wrapped in a system reminder and inserted at the
  event-appropriate point.

### Per-event input and decision control (selected)

- **SessionStart**: matcher values `startup`/`resume`/`clear`/`compact`. Input
  adds `source`, optional `model`, optional `agent_type`, `session_title`.
  Decision fields: `additionalContext`, `initialUserMessage`, `sessionTitle`,
  `watchPaths`, `reloadSkills`. Plain stdout is added as context. Only
  `command`/`mcp_tool` hooks supported.
- **UserPromptSubmit**: input adds `prompt`; `permission_mode` included. Default
  timeout 30s; on timeout the output (including `additionalContext`) is
  discarded and the prompt still reaches Claude. Decision: `decision:"block"`
  (erases prompt), `reason`, `additionalContext`, `sessionTitle`,
  `suppressOriginalPrompt`. **Plain non-JSON stdout is added as context.**
- **PreToolUse**: input adds `tool_name`, `tool_input`, `tool_use_id`. Tool
  `tool_input` field names are Claude's (Bash `command`/`description`/`timeout`
  in ms/`run_in_background`; Write/Edit/Read `file_path`; etc.). Decision inside
  `hookSpecificOutput`: `permissionDecision` (`allow`/`deny`/`ask`/`defer`),
  `permissionDecisionReason`, `updatedInput` (**replaces the entire input
  object**), `additionalContext`. Precedence across multiple hooks:
  `deny` > `defer` > `ask` > `allow`. Top-level `decision`/`reason` are
  **deprecated** (approve→allow, block→deny). `"ask"` is honoured with a real
  permission prompt; `"defer"` (v2.1.89+) pauses for SDK/headless resume.
- **PostToolUse**: input adds `tool_input`, `tool_response`, `tool_use_id`,
  `duration_ms`. Decision: `decision:"block"` (adds `reason` next to tool
  result; Claude still sees original output), `reason`, `additionalContext`,
  `updatedToolOutput` (**replaces the tool's structured output**; must match the
  tool's output shape, e.g. Bash `{stdout,stderr,interrupted,isImage}`),
  `updatedMCPToolOutput`.
- **PostToolUseFailure**: input adds `tool_name`, `tool_input`, `tool_use_id`,
  `error`, `is_interrupt`, `duration_ms`. Decision: `additionalContext` only.
- **Stop**: "Runs when the main Claude Code agent has finished responding. Does
  not run if the stoppage occurred due to a user interrupt. API errors fire
  `StopFailure` instead." Input adds `stop_hook_active`, `last_assistant_message`,
  `background_tasks`, `session_crons` (v2.1.145+), `permission_mode`. "Claude
  Code overrides the hook and ends the turn after **8 consecutive blocks**."
  Decision: `decision:"block"` (reason required; prevents stopping),
  `reason`, `hookSpecificOutput.additionalContext` (transcripted as "Stop hook
  feedback", conversation continues through the same loop protections).
- **PreCompact**: matcher `manual`/`auto`. Input adds `trigger`,
  `custom_instructions`. Exit 2 blocks compaction; `decision:"block"` also
  blocks. Blocking auto-compact before the limit just skips it; blocking
  recovery after a context-limit error surfaces the underlying error.
- **PostCompact**: matcher `manual`/`auto`. Input adds `trigger`,
  `compact_summary`. No decision control.
- **SessionEnd**: matcher/reason values `clear`/`resume`/`logout`/
  `prompt_input_exit`/`bypass_permissions_disabled`/`other`. Input adds
  `reason`. No decision control. Default timeout **1.5s**.

### Async hooks

- `"async": true` is available only on `type: "command"`. Claude starts the hook
  and continues immediately; the hook **cannot block or control** behavior
  (`decision`/`permissionDecision`/`continue` have no effect). After it exits,
  `additionalContext` is delivered on the next turn and `systemMessage` is shown
  to the user. Async default timeout is the same 10-minute sync default. Output
  is schema-validated; wrong-typed fields are dropped (v2.1.202+; earlier
  malformed JSON could crash the session).

### Prompt and agent hooks

- `type: "prompt"` sends hook input + a prompt to a fast Claude model (Haiku
  default); `$ARGUMENTS` injects the JSON. Response schema `{ok: bool, reason?}`;
  `ok:false` maps to `decision:"block"` with per-event effects; `continueOnBlock`
  optionally continues the turn. Default timeout 30.
- `type: "agent"` (experimental) spawns a subagent with tool access (up to 50
  turns) returning the same `{ok, reason}` schema. Default timeout 60.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product reference
- Surface: Claude Code hooks
- Retrieval depth: full markdown body read (hooks lifecycle, configuration,
  input/output, exit codes, every event section, prompt/agent/async sections,
  security, Windows, debug); inline version markers up to v2.1.205 observed
- Acquisition note: the rendered page is a Mintlify SPA whose HTML shell does
  not contain the article body; the markdown endpoint `.md` returns the full
  reference body, which is the form attested here
