---
source_handle: pi-hooks-source
fetched: 2026-07-12
source_path: /home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/
provenance: source-direct
substrate_confidence: source-direct
package_version: 0.0.2
---

# `@hsingjui/pi-hooks` package source (v0.0.2)

## Summary

`@hsingjui/pi-hooks` (package.json `name`, `version: "0.0.2"`) declares itself a
"Claude Code-compatible command hooks" extension for Pi (`package.json`
`description`). Its `pi.extensions` field points at a single entry
`./src/pi-hooks.ts`. The package `files` list is `["src", "README.md"]`; **no
test files ship in the package** (a `find` for `*.test.*` / `*.spec.*` under the
package root returns nothing), so behavioral claims below rest on reading the
TypeScript source, not on attested test evidence.

The entry point (`src/pi-hooks.ts`) builds a shared `HookModuleContext`
(`createHookContext`) and registers five hook modules: session, compact, prompt,
stop, and tool. Each module binds Pi events via `pi.on(...)` and translates them
into a Claude-Code-shaped hook invocation: spawn the configured command through
`bash -c`, feed a Claude-style JSON object on stdin, parse stdout JSON, and map
the result back onto Pi's event-return contract. Configuration is read from
`~/.pi/agent/settings.json` (global) and `<cwd>/.pi/settings.json` (project),
merged by concatenating per-event hook-group arrays, and matched with a
single-string regex `matcher`.

## Anchored excerpts and source-internal facts

### Package identity and entry point (`package.json`, `src/pi-hooks.ts`)

- `package.json` fields: `"name": "@hsingjui/pi-hooks"`, `"version": "0.0.2"`,
  `"type": "module"`, `pi.extensions: ["./src/pi-hooks.ts"]`, `peerDependencies`
  `"@earendil-works/pi-coding-agent": "*"`.
- `src/pi-hooks.ts` default export: builds `shared = createHookContext(pi)` then
  calls `registerSessionHooks`, `registerCompactHooks`, `registerPromptHooks`,
  `registerStopHooks`, `registerToolHooks`.

### Configuration model (`src/config.ts`, `src/types.ts`)

- `GLOBAL_SETTINGS_PATH = path.join(os.homedir(), ".pi", "agent", "settings.json")`.
  Project settings are read from `path.join(cwd, ".pi", "settings.json")`
  (`loadSettings`). There is no other scope.
- `HooksConfig` recognises nine event keys in PascalCase **and** a snake_case
  alias for each (`SessionStart`/`session_start`, …, `Stop`/`stop`).
  `getHookGroups` returns the concatenation of both aliases for the requested
  event.
- `mergeHooks(globalHooks, projectHooks)` concatenates arrays per key:
  `[...globalHooks[key], ...projectHooks[key]]`. The merged result is returned
  only when at least one group exists.
- Hook type (`src/types.ts` `Hook`): `{ type: "command"; command: string; if?:
  string; timeout?: number; async?: boolean }`. The `async` field is **declared
  on the type but never read** anywhere in `src/` (grep for `hook.async` /
  `.async` against the source returns only the type declaration and
  `async function` keywords).
- `matcherMatches(matcher, value)` (`src/config.ts`): returns `true` when
  `matcher` is falsy, `""`, or `"*"`; otherwise `new RegExp(matcher).test(value)`,
  with a `catch` fallback to `matcher === value` (exact string) on invalid
  regex. **No comma-as-alternative handling** — only the regex engine and the
  invalid-regex exact-match fallback.

### Event → Pi binding (the registration modules)

- `SessionStart` (`src/hooks/session-hooks.ts`): bound to `pi.on("session_start",
  ...)`. Fires `startup` when `event.reason === "startup" || event.reason ===
  "new"`; fires `resume` when `event.reason === "resume"`; **silently does
  nothing for `reload` or `fork` reasons** (no branch). A `firedSessionStartKeys`
  `Set` dedupes by `${matcher}:${sessionId}` so each `(matcher, session)` pair
  fires once.
- `SessionEnd` (`src/hooks/session-hooks.ts`): bound to `pi.on("session_shutdown",
  ...)`. The reason is **hardcoded to the literal `"other"`**; `event.reason` is
  ignored (the handler is `(_event, ctx) => ...`). When `matcher` is omitted on a
  `SessionEnd` group, `triggerSimpleHooks` defaults it to `"other"`.
- `PreCompact` (`src/hooks/compact-hooks.ts`): bound to
  `pi.on("session_before_compact", ...)`. `trigger` is **hardcoded to
  `"manual"`** regardless of `event.reason`; `customInstructions` is read from
  `event.customInstructions ?? ""`.
- `PostCompact` (`src/hooks/compact-hooks.ts`): bound to
  `pi.on("session_compact", ...)`. `trigger` is **hardcoded to `"manual"`**;
  `compactSummary` is read from `event.compactionEntry.summary`. After
  PostCompact runs, the handler calls `shared.triggerSessionStartHook("compact",
  ctx)`, which is how `SessionStart` matcher `compact` is produced (not from
  `session_start`).
- `UserPromptSubmit` (`src/hooks/prompt-hooks.ts`): bound to `pi.on("input",
  ...)`. The handler resets `pendingUserPromptContext` and `stopHookActive`,
  builds a context with `prompt: event.text`, and runs the hooks. On
  `decision:"block"` it returns `{ action: "handled" }`; otherwise it stores
  `additionalContext` on `shared.pendingUserPromptContext`. **It does not inspect
  `event.source`** (interactive / rpc / extension) nor guard against
  skill/template-prefixed input. A second handler, `pi.on("before_agent_start",
  ...)`, injects the pending context as a hidden `pi-hooks`-typed message.
- `Stop` (`src/hooks/stop-hooks.ts`): bound to `pi.on("agent_end", ...)`.
  `lastAssistantMessage` is derived by scanning `event.messages` backwards for
  the last `role === "assistant"` entry and extracting text. `stop_hook_active`
  is the shared boolean. On `decision:"block"` it sets `stopHookActive = true`
  and calls `pi.sendMessage({ customType: "pi-hooks", content, display: false,
  details: {...} }, { deliverAs: "followUp", triggerTurn: true })` to continue
  the turn. **There is no 8-consecutive-block cap**; loop prevention rests on the
  `stopHookActive` flag and hook self-discipline.
- `PreToolUse` / `PostToolUse` / `PostToolUseFailure` (`src/hooks/tool-hooks.ts`):
  `PreToolUse` is bound to `pi.on("tool_call", ...)`; the success/failure split
  for the two post-events is bound to a single `pi.on("tool_result", ...)`
  handler that branches on `event.isError`. `toolName` and `toolInput` come from
  `event.toolName` / `event.input`; `toolUseId` from `event.toolCallId`.

### `if` condition evaluation (`src/hooks/shared.ts`)

- `hookIfMatches(context, condition)` returns `false` unless `hookEventName` is
  `PreToolUse` / `PostToolUse` / `PostToolUseFailure` (so `if` on any other event
  silently disables the hook). The condition is parsed with the regex
  `/^([^()]+?)(?:\((.*)\))?$/` into `ToolName(pattern)`. ToolName is compared
  case-insensitively; `pattern` uses **simple wildcard globbing** where `*` →
  `.*` (via `globToRegex`, anchored `^...$`, case-insensitive).
- `getToolInputMatchValue(toolName, toolInput)` picks the field the pattern
  matches against: `bash` → `command`; `read`/`write`/`edit` → `path` then
  `file_path`; `grep`/`find`/`glob` → `pattern` then `path`; `ls` → `path`;
  otherwise the JSON string of `toolInput`.

### Executor and JSON parsing (`src/executor.ts`, `src/hooks/shared.ts`)

- `executeCommandHook` spawns `spawn("bash", ["-c", command], { cwd, stdio:
  ["pipe","pipe","pipe"] })`, writes the JSON input to stdin, and resolves with
  `{ stdout, stderr, exitCode }`. On timeout the child is killed and the result
  is forced to `exitCode: 1` with an appended `[pi-hooks] Hook timed out` line.
- Default timeout is `60000` ms (`executeHook(hook, input, cwd, timeoutMs =
  60000)`); `executeParsedHook` passes `hook.timeout ? hook.timeout * 1000 :
  60000`. The default is uniform across all events; the only override path is a
  per-hook `timeout` field.
- `executeParsedHook` parses stdout as JSON only when non-empty; `extractCommonOutput`
  reads `hookSpecificOutput` (validated by `hookEventName` match), `systemMessage`,
  `suppressOutput`, `stopProcessing` (`continue === false`), and `stopReason`.

### Exit-code handling per event (as implemented)

- `PreToolUse` (`triggerPreToolUseHooks`): `exitCode === 2` → blocked with
  `reason = stderr || "Blocked by hook"`. `exitCode === 0` with JSON honours
  `permissionDecision` `deny`/`allow`/`ask`, `updatedInput`, `additionalContext`,
  and `stopProcessing`. Other non-zero exit codes are notified as errors.
- `PostToolUse` / `PostToolUseFailure` (`trigger*Hooks`): `exitCode === 2` →
  notifies "反馈" (feedback) as a warning and **continues** (does not feed stderr
  to Claude as a structured tool-result side channel beyond the notify); the
  loop is not aborted for exit 2.
- `UserPromptSubmit` / `Stop` (`trigger*Hooks`): only JSON `decision === "block"`
  blocks; `exitCode === 2` falls into the generic `exitCode !== 0` error-notify
  branch and does **not** block.
- `SessionStart` / `SessionEnd` / `PreCompact` / `PostCompact` (via
  `triggerSimpleHooks`): no exit-2 special-casing; non-zero exit notifies an
  error. For `SessionStart`, plain non-JSON stdout on exit 0 is appended to
  `additionalContext`; for other events plain stdout is surfaced via an info
  notify (unless `suppressOutput`).

### `updatedInput` and tool-result patching (as implemented)

- `PreToolUse` `updatedInput` is applied with `Object.assign(event.input,
  result.updatedInput)` — a **merge**, not a replace, into `event.input`.
- `PostToolUse` / `PostToolUseFailure` patching reads
  `hookSpecificOutput.updatedToolResult.{content,details,isError}` (preferred),
  then `hookSpecificOutput.updatedMCPToolOutput` / top-level `updatedMCPToolOutput`
  (for `content` only), then top-level `content` / `details` / `isError`. The
  returned patch from the `tool_result` handler replaces `event.content` /
  `event.details` / `event.isError` when supplied. The field name read is
  `updatedToolResult`, not `updatedToolOutput`.
- `PreToolUse` `permissionDecision: "ask"` is acknowledged in code but the
  README states it is "kept for compatibility only; this extension does not open
  an additional permission UI".

## Structural metadata

- Publisher: hsingjui (npm `@hsingjui/pi-hooks`, MIT)
- Document type: installed package source (TypeScript, ESM)
- Surface: Pi extension implementing a Claude-Code-style hook shim
- Retrieval depth: full source read (`src/pi-hooks.ts`, `config.ts`,
  `executor.ts`, `helpers.ts`, `hook-context.ts`, `types.ts`, and all five files
  under `src/hooks/`); `README.md` read for cross-checking; no tests present
- Notable absence: no test files in the published package
