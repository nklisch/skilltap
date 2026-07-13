---
source_handle: pi-extension-events
fetched: 2026-07-12
source_path: /home/nathan/.local/share/mise/installs/node/24.17.0/lib/node_modules/@earendil-works/pi-coding-agent/docs/extensions.md
provenance: source-direct
substrate_confidence: source-direct
sdk_version: 0.80.6
---

# Pi extension event reference (SDK v0.80.6, `docs/extensions.md`)

## Summary

The Pi SDK documentation `docs/extensions.md` (installed with
`@earendil-works/pi-coding-agent` v0.80.6) is the authoritative reference for
the `pi.on(event, handler)` extension event system. Handlers receive
`(event, ctx: ExtensionContext)`. Events are grouped into Session, Agent, Tool,
User Bash, and Input families, plus provider/model events. Several events
document explicit return-value contracts that control blocking, message
injection, and result modification. The events pi-hooks binds to are
`session_start`, `session_shutdown`, `session_before_compact`,
`session_compact`, `before_agent_start`, `agent_end`, `tool_call`,
`tool_result`, and `input`; their documented semantics are recorded below.

`ctx` exposes `ctx.cwd`, `ctx.ui.notify(msg, type)`, `ctx.sessionManager`
(including `getSessionFile()`), `ctx.abort?.()`, `ctx.signal`, `ctx.mode`
(`"tui"`/`"rpc"`/`"json"`/`"print"`), and `ctx.hasUI`.

## Anchored excerpts and source-internal facts

### `session_start` (extensions.md §Session Events)

- "Fired when a session is started, loaded, or reloaded."
- `event.reason` is documented as `"startup" | "reload" | "new" | "resume" |
  "fork"` (comment on the example handler, line 397).
- `event.previousSessionFile` is present for `"new"`, `"resume"`, and `"fork"`.
- After a successful `/new` or `/resume` switch, Pi emits `session_shutdown` for
  the old extension instance, reloads and rebinds extensions, then emits
  `session_start` with `reason: "new" | "resume"` and `previousSessionFile`.
- The same reload-and-rebind sequence is documented for `/fork` and `/clone`,
  emitting `session_start` with `reason: "fork"`.

### `session_shutdown` (extensions.md §Session Events)

- "Fired before a started session runtime is torn down." Recommended for
  cleanup of resources opened in `session_start`.
- `event.reason` is documented as `"quit" | "reload" | "new" | "resume" |
  "fork"`; `event.targetSessionFile` is the destination session for replacement
  flows.
- The doc explicitly tells authors: "Do cleanup work in `session_shutdown`, then
  reestablish any in-memory state in `session_start`."

### `session_before_compact` / `session_compact` (extensions.md §Session Events)

- `session_before_compact` event fields: `preparation`, `branchEntries`,
  `customInstructions`, `reason`, `willRetry`, `signal`. `reason` is
  `"manual"` (`/compact`), `"threshold"`, or `"overflow"`; `willRetry` reports
  whether the aborted turn is retried after compaction (overflow recovery). A
  handler can `return { cancel: true }` to cancel, or return a `compaction`
  object with a custom `summary`, `firstKeptEntryId`, `tokensBefore`.
- `session_compact` event fields: `compactionEntry` (the saved compaction),
  `fromExtension`, `reason` (`"manual"`/`"threshold"`/`"overflow"`),
  `willRetry`.

### `before_agent_start` (extensions.md §Agent Events)

- "Fired after user submits prompt, before agent loop. Can inject a message
  and/or modify the system prompt."
- `event.prompt`, `event.images`, `event.systemPrompt` (chained across
  handlers), and `event.systemPromptOptions` (structured `.customPrompt`,
  `.selectedTools`, `.toolSnippets`, `.promptGuidelines`, `.appendSystemPrompt`,
  `.cwd`, `.contextFiles`, `.skills`).
- Return shape: `{ message: { customType, content, display }, systemPrompt }`
  to inject a persistent message and/or replace the system prompt for the turn.

### `agent_start` / `agent_end` / `agent_settled` (extensions.md §Agent Events)

- "`agent_start` fires when a low-level agent run begins. `agent_end` fires when
  that run ends, but Pi may still auto-retry, auto-compact and retry, or continue
  with queued follow-up messages. Use `agent_settled` for status integrations
  that need to know Pi will not continue running automatically."
- `agent_end` event field: `event.messages` — "messages from this low-level
  run".
- `agent_settled`: "`ctx.isIdle()` is true here unless another extension started
  a new run."

### `tool_call` (extensions.md §Tool Events)

- "Fired after `tool_execution_start`, before the tool executes. **Can block.**"
- "Before `tool_call` runs, pi waits for previously emitted Agent events to
  finish draining through `AgentSession`."
- In parallel tool mode, sibling tool calls are preflighted sequentially then
  executed concurrently; "`tool_call` is not guaranteed to see sibling tool
  results from that same assistant message in `ctx.sessionManager`."
- `event.toolName` ("bash", "read", "write", "edit", etc.), `event.toolCallId`,
  `event.input` — **mutable**.
- Behavior guarantees: mutations to `event.input` affect execution; later
  handlers see earlier mutations; **no re-validation** after mutation; return
  values control blocking only via `{ block: true, reason?: string }`.

### `tool_result` (extensions.md §Tool Events)

- "Fired after tool execution finishes and before `tool_execution_end` plus the
  final tool result message events are emitted. **Can modify result.**"
- In parallel mode, `tool_result` and `tool_execution_end` may interleave in
  completion order; final `toolResult` message events still emit later in
  assistant source order.
- Handlers chain like middleware: run in extension load order; each sees the
  latest result; a handler may **return partial patches** `{ content, details,
  isError }`; omitted fields keep current values. `ctx.signal` is available for
  nested abort-aware async work.
- `event.toolName`, `event.toolCallId`, `event.input`, `event.content`,
  `event.details`, `event.isError`. The handler may dispatch on `event.isError`
  to distinguish success from failure.

### `input` (extensions.md §Input Events)

- "Fired when user input is received, after extension commands are checked but
  before skill and template expansion. The event sees the raw input text, so
  `/skill:foo` and `/template` are not yet expanded."
- Documented processing order: (1) extension commands `/cmd` checked first; if
  found, handler runs and `input` is skipped; (2) `input` fires; (3) if not
  handled, skill commands `/skill:name` expanded; (4) prompt templates
  `/template` expanded; (5) agent processing begins (`before_agent_start`, …).
- `event.text` (raw input), `event.images`, `event.source` (`"interactive"` /
  `"rpc"` / `"extension"`), `event.streamingBehavior` (`"steer"` / `"followUp"`
  / `undefined`).
- Documented return actions: `{ action: "continue" }` (pass through, default),
  `{ action: "transform", text, ... }` (rewrite then continue to expansion),
  `{ action: "handled" }` (skip agent entirely; first handler to return this
  wins). Transforms chain across handlers.

## Structural metadata

- Publisher: Pi / Earendil (`@earendil-works/pi-coding-agent`, installed v0.80.6)
- Document type: product / SDK reference
- Surface: Pi extension event system
- Retrieval depth: targeted read of every event section pi-hooks binds to
  (Session, Agent, Tool, Input families) plus the `ExtensionContext` fields
  referenced by pi-hooks (`ctx.cwd`, `ctx.ui.notify`, `ctx.sessionManager`,
  `ctx.abort`); supporting provider/model and user_bash sections not in scope
- Note: Pi SDK docs are read from the on-disk installed package, not a web URL;
  the version pin (0.80.6) is read from the installed `package.json`
