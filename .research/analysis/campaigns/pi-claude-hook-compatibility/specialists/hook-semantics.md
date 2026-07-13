---
provenance: agent-synthesis
updated: 2026-07-12
facet: hook-semantics
campaign: pi-claude-hook-compatibility
temporal_contract: supersedes-prior
sources:
  - pi-hooks-source
  - pi-extension-events
  - claude-hooks-reference
---

# Hook event, payload, ordering, blocking, and failure semantics

This brief answers facet 2 of the `pi-claude-hook-compatibility` campaign:
**event-by-event semantic equivalence between current Claude Code hooks and the
`@hsingjui/pi-hooks` Pi extension**, covering timing, payload, matcher behavior,
environment, working directory, permission semantics, exit/blocking behavior,
async behavior, and unsupported events. It does not adjudicate package identity
or health/ownership (facets 1 and 3); it reports only what the source and the two
contract references establish.

## Source map

| Number | Handle | Source |
|---:|---|---|
| 1 | `pi-hooks-source` | installed package source `@hsingjui/pi-hooks` v0.0.2 (`src/`) [pi-hooks-source]{1} |
| 2 | `pi-extension-events` | Pi SDK v0.80.6 `docs/extensions.md` event reference [pi-extension-events]{2} |
| 3 | `claude-hooks-reference` | Anthropic "Hooks reference", `https://code.claude.com/docs/en/hooks` [claude-hooks-reference]{3} |

All three were read source-direct. The pi-hooks package ships **no tests**
[pi-hooks-source]{1}, so behavioral claims about the extension rest on the
TypeScript source, not on attested test evidence; this is recorded as an
evidence qualifier throughout.

## Facet verdict

`@hsingjui/pi-hooks` v0.0.2 is a **partial, best-effort** Claude-Code hook
shim, not a faithful implementation of the Claude hook contract. It covers nine
of Claude's roughly thirty hook events [claude-hooks-reference]{3} and only the
`command` hook type [pi-hooks-source]{1}; within those nine, divergence is the
norm rather than the exception across payload fields, matcher semantics, exit
codes, timeouts, the `async` flag, blocking safety caps, and several
decision-control fields. A handful of paths are faithful (SessionStart plain
stdout → context, the PreToolUse `exit 2` block, the `matcher`/`""`/`"*"`-means-all
rule, tool-result middleware patching). The extension's own README and source
both describe the mapping as best-effort [pi-hooks-source]{1}. {inferred:
aggregates} No single divergence is automatically disqualifying, but several
(PreToolUse `updatedInput` merge-vs-replace, the ignored `async` flag, the
absent Stop 8-block cap, SessionEnd's 60s-vs-1.5s timeout, the `updatedToolOutput`
field-name mismatch, and tool-name casing) break portability for hooks that
exercise the documented Claude surface. A compound Pi adapter may not treat
this extension as a faithful Claude-hooks target without an explicit, narrowed
compatibility contract; the contract must enumerate the divergences below.

## Compatibility matrix

Legend: **F** = faithful (same observable contract), **P** = partial (works for
a subset / with caveats), **X** = divergent (different observable behavior),
**—** = unsupported. "Notes" cite the specific divergence.

### Event coverage

| Claude event | pi-hooks | Status | Notes |
|---|---|---|---|
| `SessionStart` | yes | P | only `command`; matchers `startup`/`resume`/`compact`; **no `clear`**; Pi reasons `reload`/`fork`/`new` collapsed or dropped [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| `SessionEnd` | yes | X | always fires; **`reason` hardcoded to `"other"`**; Pi reasons `quit`/`reload`/`new`/`resume`/`fork` ignored [pi-hooks-source]{1} [pi-extension-events]{2} [claude-hooks-reference]{3} |
| `UserPromptSubmit` | yes | P | bound to Pi `input` (pre skill/template expansion, no `source` guard); only `decision:"block"` blocks; **exit 2 does not block**; plain stdout not added as context [pi-hooks-source]{1} [pi-extension-events]{2} [claude-hooks-reference]{3} |
| `PreToolUse` | yes | P | bound to Pi `tool_call`; `exit 2` blocks (faithful); `permissionDecision` `deny`/`allow`/`ask`; **no `defer`**; `updatedInput` **merges** (Claude replaces); deprecated top-level `decision` not honored [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| `PostToolUse` | yes | P | bound to Pi `tool_result` (success only); honors `additionalContext`, `decision:block`→context, Pi result patching; **field `updatedToolOutput` not read** (reads `updatedToolResult`); `duration_ms` absent [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| `PostToolUseFailure` | yes | P | bound to Pi `tool_result` (`isError`); `error` provided, `is_interrupt` hardcoded `false`, `duration_ms` absent; `additionalContext` honored [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| `Stop` | yes | X | bound to Pi `agent_end` (a low-level-run-end event that can fire multiple times per logical response and on retry/compact-retry); **no 8-consecutive-block cap**; **no interrupt exclusion**; `StopFailure` not separated; `background_tasks`/`session_crons`/`permission_mode` absent [pi-hooks-source]{1} [pi-extension-events]{2} [claude-hooks-reference]{3} |
| `PreCompact` | yes | X | bound to Pi `session_before_compact`; **`trigger` hardcoded `"manual"`**; **cannot block** (exit 2 and `decision:"block"` not honored) [pi-hooks-source]{1} [pi-extension-events]{2} [claude-hooks-reference]{3} |
| `PostCompact` | yes | P | bound to Pi `session_compact`; `compact_summary` provided; **`trigger` hardcoded `"manual"`** [pi-hooks-source]{1} [pi-extension-events]{2} [claude-hooks-reference]{3} |
| `Setup` | no | — | not surfaced (no Pi equivalent bound) [pi-hooks-source]{1} |
| `StopFailure` | no | — | folded into `Stop` on `agent_end` (Pi emits no distinct failure event the extension binds) [pi-hooks-source]{1} [pi-extension-events]{2} |
| `SubagentStart` / `SubagentStop` | no | — | not surfaced; `agent_end` fires for subagent runs without `agent_type` distinction [pi-extension-events]{2} |
| `PostToolBatch` | no | — | not surfaced [pi-hooks-source]{1} |
| `PermissionRequest` / `PermissionDenied` | no | — | not surfaced (Pi permission model differs) [pi-hooks-source]{1} |
| `Notification`, `MessageDisplay`, `UserPromptExpansion` | no | — | not surfaced [pi-hooks-source]{1} |
| `TaskCreated`, `TaskCompleted`, `TeammateIdle` | no | — | not surfaced [pi-hooks-source]{1} |
| `ConfigChange`, `CwdChanged`, `FileChanged`, `InstructionsLoaded` | no | — | not surfaced [pi-hooks-source]{1} |
| `WorktreeCreate`, `WorktreeRemove`, `Elicitation`, `ElicitationResult` | no | — | not surfaced [pi-hooks-source]{1} |

### Cross-cutting contract dimensions

| Dimension | Claude | pi-hooks | Status | Notes |
|---|---|---|---|---|
| Hook types supported | `command`, `http`, `mcp_tool`, `prompt`, `agent` [claude-hooks-reference]{3} | `command` only [pi-hooks-source]{1} | X | README names http/prompt/agent unsupported; `mcp_tool` unsupported but not called out [pi-hooks-source]{1} |
| `matcher` semantics | single regex; alternatives via `\|` **or `,`** [claude-hooks-reference]{3} | single regex; `""`/`"*"`/omitted = all; invalid regex → exact match; **no `,` alternative** [pi-hooks-source]{1} | P | a matcher using `,` works in Claude and silently misbehaves in pi-hooks |
| `if` syntax | permission-rule syntax (`Bash(git *)`), tool events only [claude-hooks-reference]{3} | `ToolName(pattern)`, `*`→`.*` glob, case-insensitive, tool events only [pi-hooks-source]{1} | P | pi-hooks `if` is far simpler than Claude's permission rules (no `$()`/backtick handling); tool-event-only restriction matches |
| Common input fields | `session_id`, `prompt_id`, `transcript_path`, `cwd`, `permission_mode`, `effort`, `hook_event_name` (+ `agent_id`/`agent_type`) [claude-hooks-reference]{3} | `session_id`, `transcript_path`, `cwd`, `hook_event_name` [pi-hooks-source]{1} | X | **`permission_mode`, `prompt_id`, `effort`, `agent_id`, `agent_type` omitted**; README admits `permission_mode` excluded [pi-hooks-source]{1} |
| Tool-name casing in matchers/`tool_input` | capitalised (`Bash`, `Edit`, `Write`, `Read`, …); `tool_input.file_path` etc. [claude-hooks-reference]{3} | Pi's raw lowercase names (`bash`, `read`, …); Pi's native `tool_input` field names [pi-hooks-source]{1} | X | a Claude matcher `Bash` won't match pi-hooks' `bash` (case-sensitive regex); a Claude hook reading `tool_input.file_path` sees Pi's `path` |
| `model` field (SessionStart) | supplied to SessionStart only, not guaranteed [claude-hooks-reference]{3} | declared in input builder but **always undefined** (never populated) [pi-hooks-source]{1} | X | SessionStart `model` is effectively absent |
| Working directory | hook runs in session `cwd` [claude-hooks-reference]{3} | spawn `cwd` set to `ctx.cwd` [pi-hooks-source]{1} | F | faithful |
| Execution shell | `bash` (or `shell:"powershell"` on Windows) [claude-hooks-reference]{3} | always `bash -c` [pi-hooks-source]{1} | P | Windows PowerShell path unsupported; `${CLAUDE_PROJECT_DIR}`/`${CLAUDE_PLUGIN_ROOT}`/`${CLAUDE_PLUGIN_DATA}` placeholders not rewritten [pi-hooks-source]{1} |
| Environment | inherits parent env; `CLAUDE_ENV_FILE` for SessionStart/Setup/CwdChanged/FileChanged [claude-hooks-reference]{3} | inherits parent env; **no `CLAUDE_ENV_FILE`** support [pi-hooks-source]{1} | X | env-persistence hooks have no Pi analogue |
| Default timeout | 600s (command/http/mcp_tool); UserPromptSubmit→30s; MessageDisplay→10s; SessionEnd→**1.5s**; prompt 30s; agent 60s [claude-hooks-reference]{3} | **uniform 60s**; only per-hook `timeout` overrides [pi-hooks-source]{1} | X | SessionEnd is the widest gap (60s vs 1.5s); UserPromptSubmit also differs (60s vs 30s) |
| Exit code 2 (block) | per-event blocking semantics; PostToolUse/PostToolUseFailure exit 2 → stderr to Claude; UserPromptSubmit/Stop/PreCompact exit 2 blocks [claude-hooks-reference]{3} | only `PreToolUse` honours exit 2 as block; PostToolUse/Failure exit 2 → warn-and-continue; UserPromptSubmit/Stop exit 2 → error-notify, **no block**; PreCompact exit 2 → error-notify [pi-hooks-source]{1} | X | exit-2 blocking is largely unimplemented outside PreToolUse |
| JSON output: universal fields | `continue`, `stopReason`, `suppressOutput`, `systemMessage`, `terminalSequence` [claude-hooks-reference]{3} | `continue`(=stopProcessing), `stopReason`, `suppressOutput`, `systemMessage`; **no `terminalSequence`** [pi-hooks-source]{1} | P | `terminalSequence` (OSC notifications) unsupported |
| Output 10k cap | strings capped at 10,000 chars; overflow → file [claude-hooks-reference]{3} | **no cap** [pi-hooks-source]{1} | X | large `additionalContext` injected verbatim |
| `async` flag | command-only; runs in background; cannot block; output delivered next turn [claude-hooks-reference]{3} | **declared on `Hook` type but never read**; executor always awaits [pi-hooks-source]{1} | X | an `async:true` hook **blocks** in pi-hooks — the inverse of the Claude contract |
| `additionalContext` delivery | wrapped as system reminder at event-appropriate point; Stop feedback transcripted as "Stop hook feedback" [claude-hooks-reference]{3} | injected as hidden `pi-hooks`-typed message (`display:false`) via `sendMessage`/`before_agent_start` [pi-hooks-source]{1} | P | same intent (context to the model), different visibility and delivery mechanism |
| PreToolUse `updatedInput` | **replaces entire input object** [claude-hooks-reference]{3} | `Object.assign` **merge** [pi-hooks-source]{1} | X | a hook returning partial input keeps unspecified fields in pi-hooks, wipes them in Claude |
| PreToolUse `permissionDecision:"ask"` | real permission prompt [claude-hooks-reference]{3} | acknowledged but **no permission UI** (no-op) [pi-hooks-source]{1} | X | `"ask"` behaves like `"allow"` in effect |
| PostToolUse `updatedToolOutput` | replaces structured tool output [claude-hooks-reference]{3} | **field not read** (reads `updatedToolResult` + Pi patch fields) [pi-hooks-source]{1} | X | field-name mismatch; a Claude hook using `updatedToolOutput` has no effect |
| Stop continuation cap | overrides after 8 consecutive blocks [claude-hooks-reference]{3} | **no cap**; relies on `stop_hook_active` + hook self-discipline [pi-hooks-source]{1} | X | a hook that always blocks can loop indefinitely |
| Stop interrupt/failure split | Stop excluded on user interrupt; `StopFailure` for API errors [claude-hooks-reference]{3} | bound to `agent_end` regardless; no failure split; `agent_end` may fire multiple times per response [pi-hooks-source]{1} [pi-extension-events]{2} | X | Stop can fire on the wrong boundary (retry, compact-retry, follow-up, API-error turn) |
| Tool-result middleware patch | `tool_result` returns partial `{content,details,isError}` patches [pi-extension-events]{2} | returns same partial patch shape [pi-hooks-source]{1} | F | the Pi-side patch contract is faithfully relayed |

## Detailed divergence notes

The matrix above is load-bearing; the notes below record the mechanism for the
non-obvious rows so the synthesizer can adjudicate without re-reading source.

**SessionStart reason collapse.** Pi's `session_start` carries `reason` in
`"startup" | "reload" | "new" | "resume" | "fork"` [pi-extension-events]{2}.
pi-hooks fires the Claude `startup` matcher for both `"startup"` and `"new"`,
fires `resume` for `"resume"`, and **silently does nothing for `"reload"` or
`"fork"`** [pi-hooks-source]{1}. Claude's `SessionStart` matcher set is
`startup`/`resume`/`clear`/`compact` [claude-hooks-reference]{3}; pi-hooks
omits `clear` entirely and synthesises `compact` from the `session_compact`
handler rather than from `session_start` [pi-hooks-source]{1}. A hook keyed on
`clear` never fires; a `/reload` or `/fork` produces no SessionStart at all.

**Stop vs `agent_end`.** Claude's `Stop` is defined to run when "the main
Claude Code agent has finished responding," is **excluded on user interrupt**,
and is replaced by `StopFailure` on API error [claude-hooks-reference]{3}. Pi's
`agent_end` "fires when [a low-level] run ends, but Pi may still auto-retry,
auto-compact and retry, or continue with queued follow-up messages"
[pi-extension-events]{2}, and the doc explicitly directs status integrations
that need "Pi will not continue running automatically" to `agent_settled`
instead [pi-extension-events]{2}. pi-hooks binds Stop to `agent_end`
[pi-hooks-source]{1}, so a Stop hook may fire on retries, compaction-retries,
follow-up continuations, and API-error turns, with no `StopFailure` separation
and no interrupt exclusion. {inferred: composes} This is the single largest
timing/semantic gap in the facet.

**`async` is dead.** The `Hook` type declares `async?: boolean`
[pi-hooks-source]{1}, matching Claude's command-only async flag
[claude-hooks-reference]{3}. But `grep` of `src/` finds no read of `hook.async`;
`executeParsedHook` always `await`s `executeHook` [pi-hooks-source]{1}. A hook
ported from Claude with `"async": true` will block the turn in pi-hooks — the
opposite of its intent.

**SessionEnd reason and timeout.** The `session_shutdown` handler hardcodes
`reason = "other"` and ignores Pi's `event.reason` (`quit`/`reload`/`new`/
`resume`/`fork`) [pi-hooks-source]{1} [pi-extension-events]{2}. Claude's
`SessionEnd` matcher/reason set is `clear`/`resume`/`logout`/`prompt_input_exit`/
`bypass_permissions_disabled`/`other` [claude-hooks-reference]{3}, so a hook
matching anything but `other` never fires. The default timeout is 60s in
pi-hooks vs **1.5s** in Claude [claude-hooks-reference]{3} [pi-hooks-source]{1};
a SessionEnd hook that relies on Claude's tight budget behaves very differently.

**`UserPromptSubmit` timing and exit codes.** pi-hooks binds UserPromptSubmit to
Pi's `input` event, which fires **before skill and template expansion** and
carries `event.source` in `interactive`/`rpc`/`extension`
[pi-extension-events]{2}. The handler does not inspect `event.source`, so
extension-injected or RPC inputs would also trigger UserPromptSubmit
[pi-hooks-source]{1}. Claude treats UserPromptSubmit's **plain stdout as
context** and **exit 2 as a block** [claude-hooks-reference]{3}; pi-hooks does
neither — plain stdout becomes an info notification, and exit 2 falls into the
generic error-notify branch [pi-hooks-source]{1}. Only JSON `decision:"block"`
blocks.

**PreToolUse `updatedInput` merge-vs-replace and the `updatedToolOutput` field
name.** Claude's `updatedInput` "replaces the entire input object"
[claude-hooks-reference]{3}; pi-hooks does `Object.assign(event.input,
result.updatedInput)` [pi-hooks-source]{1} — a merge. Claude's PostToolUse
`updatedToolOutput` replaces the structured tool output
[claude-hooks-reference]{3}; pi-hooks reads `hookSpecificOutput.updatedToolResult`
(and `updatedMCPToolOutput`/top-level `content`), **not** `updatedToolOutput`
[pi-hooks-source]{1}. Both are silent incompatibilities: the Claude-shaped hook
runs without error but has the wrong effect.

**Tool-name casing and `tool_input` field names.** Claude matchers and
`tool_input` use capitalised tool names and Claude field names (`Bash`,
`file_path`, Bash `timeout` in **milliseconds**) [claude-hooks-reference]{3}.
pi-hooks matches against Pi's raw lowercase `toolName` and forwards Pi's native
`tool_input` (e.g. `path`, not `file_path`) [pi-hooks-source]{1}. A Claude
matcher `Bash` is a case-sensitive no-match against `bash`; a hook reading
`tool_input.file_path` reads `undefined`.

## Disconfirming analysis

| Load-bearing proposition tested | Disconfirming search across attested sources | Outcome |
|---|---|---|
| pi-hooks supports all Claude hook events | Cross-checked the nine pi-hooks events against Claude's full event set | Rejected; ~21 Claude events are unsupported [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| pi-hooks `async` runs hooks in the background | Searched `src/` for any read of `hook.async` / `.async` on the hook object | Rejected; the flag is declared and ignored; hooks block [pi-hooks-source]{1} |
| pi-hooks honours exit code 2 as a block on all blocking events | Checked each `trigger*Hooks` for an `exitCode === 2` branch | Rejected; only `PreToolUse` blocks on exit 2 [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| pi-hooks `Stop` is faithful to Claude `Stop` | Compared the Pi `agent_end` boundary to Claude's Stop boundary | Rejected; `agent_end` is a low-level-run-end event that can repeat and is not interrupt/failure split [pi-extension-events]{2} [claude-hooks-reference]{3} |
| pi-hooks SessionEnd relays Pi's shutdown reason | Read the `session_shutdown` handler | Rejected; reason is hardcoded `"other"` [pi-hooks-source]{1} [pi-extension-events]{2} |
| pi-hooks honours `updatedToolOutput` (the Claude field) | Searched the patch extraction code for the field name | Rejected; it reads `updatedToolResult`, not `updatedToolOutput` [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| pi-hooks `PreCompact` can block compaction | Checked whether exit 2 / `decision:"block"` is honoured | Rejected; `triggerSimpleHooks` has no block path [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| pi-hooks SessionStart is faithful | Compared matcher sets and reason mapping | Qualified-faithful on `startup`/`resume`/`compact`-via-PostCompact and plain-stdout-as-context; rejected on `clear`, `reload`, `fork`, `model`, and `reloadSkills`/`watchPaths`/`sessionTitle`/`initialUserMessage` [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| pi-hooks forwards `permission_mode` | Checked `buildHookInput` field construction | Rejected; the field is absent [pi-hooks-source]{1} [claude-hooks-reference]{3} |
| The pi-hooks package has tests corroborating the source | Ran `find` for `*.test.*`/`*.spec.*` under the package root | Rejected; no tests ship in the package; behavioral claims rest on source alone [pi-hooks-source]{1} |

## Contradictions

No two **sources** contradict each other on a shared claim: the Pi event
reference, the Claude hooks reference, and the pi-hooks source each describe
their own surface consistently. The contradictions below are between the
**Claude contract** and the **pi-hooks implementation**, surfaced as
side-by-side positions rather than resolved.

- **Stop boundary** — relationship `incommensurable`. Claude `Stop` is a
  logical-response-complete event with interrupt exclusion and a `StopFailure`
  split [claude-hooks-reference]{3}; Pi `agent_end` is a low-level-run-end event
  that Pi may re-fire [pi-extension-events]{2}. pi-hooks binds one to the other
  [pi-hooks-source]{1}. The two events cannot be stated in a shared frame
  without admitting that the mapping is lossy; forcing `contradicts` would
  falsely claim commensurability.
- **`async` flag** — relationship `contradicts`. Claude's `async:true` command
  hook runs detached and cannot block [claude-hooks-reference]{3}; pi-hooks
  parses the flag and blocks on the result [pi-hooks-source]{1}. Same input
  field, opposite observable behavior.
- **`updatedInput`** — relationship `contradicts`. Claude replaces the input
  object; pi-hooks merges it [claude-hooks-reference]{3} [pi-hooks-source]{1}.
- **Exit code 2 outside PreToolUse** — relationship `contradicts`. Claude
  blocks on exit 2 for UserPromptSubmit/Stop/PreCompact and feeds stderr to
  Claude for PostToolUse/PostToolUseFailure [claude-hooks-reference]{3};
  pi-hooks neither blocks nor feeds stderr for those events on exit 2
  [pi-hooks-source]{1}.
- **SessionEnd `reason`** — relationship `qualifies`. Claude exposes six
  reasons; pi-hooks emits exactly one (`other`) [claude-hooks-reference]{3}
  [pi-hooks-source]{1}. The Claude surface is a strict superset; pi-hooks
  collapses it to a single value.
- **`Stop` 8-block cap** — relationship `qualifies`. Claude enforces a cap and
  ends the turn after 8 consecutive blocks [claude-hooks-reference]{3}; pi-hooks
  enforces no cap [pi-hooks-source]{1}. The pi-hooks surface omits a Claude
  safety property.

## Unknowns

- {ambiguous: pi-tool_input-field-names} The Pi `tool_input` field names for
  each built-in tool (e.g. whether Pi's `read`/`write`/`edit` use `path` vs
  `file_path`, and Pi's `grep` field names) are inferred from pi-hooks'
  `getToolInputMatchValue` switch [pi-hooks-source]{1}, not read from a Pi tool
  schema. A Pi tool-input schema reference would let the matrix cite Pi field
  names directly.
- {ambiguous: pi-permission-mode-model} Whether Pi exposes a `permission_mode`
  analogue at all is not established by these sources; the omission may be a
  fundamental frame difference rather than a gap.
- {ambiguous: agent_end-on-interrupt} The Pi reference does not state whether
  `agent_end` fires when a run is aborted by user interrupt; Claude `Stop`
  explicitly excludes it [claude-hooks-reference]{3}. This affects whether
  pi-hooks Stop fires on interrupts.
- {confidence: local-source-only} The pi-hooks source and the Pi SDK doc were
  read from the locally installed artifacts (v0.0.2 and v0.80.6 respectively);
  upstream `git` history, issue tracker, and any later published versions were
  not engaged [pi-hooks-source]{1}.

## Revisit if

- `@hsingjui/pi-hooks` publishes a version that reads `hook.async`, honours
  exit 2 outside PreToolUse, relays SessionEnd reasons, caps Stop blocks, or
  renames `updatedToolResult` to `updatedToolOutput`.
- Pi adds an `agent_settled`-backed or interrupt-aware Stop analogue that
  pi-hooks binds instead of `agent_end`.
- Anthropic publishes a machine-readable JSON schema for hook input/output (the
  reference is prose with inline version markers up to v2.1.205).
- Pi publishes a tool-input schema that pins Pi `tool_input` field names and
  units (notably Bash `timeout` seconds-vs-ms).
- The commissioning item's hook-equivalence contract is narrowed to a specific
  subset of events/fields, which would re-scope this matrix from "faithful?"
  to "covers the contracted subset?".

## Acquisition candidates

- **`blocking`** — none. All three load-bearing sources were fetched source-direct
  (the Claude reference via its `.md` endpoint after the SPA shell proved
  unextractable). No cited claim is held on an unfetchable source.
- **`enriching`** — two, each grounded in a fetched source that names it:
  1. **Pi tool-input schema** (the per-tool `event.input` field names and
     units). Source: the Pi SDK `docs/` tree bundled with v0.80.6 names
     `docs/sdk.md` and `docs/session-format.md` as the message/tool-result
     reference [pi-extension-events]{2}. Class: `primary-doc`. Web-availability:
     likely on `pi.dev/docs` but not fetched. Completes: the
     {ambiguous: pi-tool_input-field-names} unknown; would let the matrix cite
     Pi field names and units (Bash timeout s-vs-ms) directly rather than via
     pi-hooks' switch.
  2. **`@hsingjui/pi-hooks` upstream repository and changelog** (`git+https://github.com/hsingjui/pi-hooks.git`,
     named in `package.json`) [pi-hooks-source]{1}. Class: `portal`.
     Web-availability: not fetched (no web tool in this dispatch context).
     Completes: version history, open issues on the divergences above, and any
     tests that exist upstream but do not ship in the npm package; would lift
     the {confidence: local-source-only} qualifier and confirm whether
     divergences are known/tracked.

## Handles

- `pi-hooks-source`{1} — `attestation/pi-hooks-source.md`
- `pi-extension-events`{2} — `attestation/pi-extension-events.md`
- `claude-hooks-reference`{3} — `attestation/claude-hooks-reference.md`
