# Pattern: Options With Callbacks

Typed options interfaces carry optional callback props that create an event-streaming pipeline from subprocess output up through the orchestrator to UI consumers.

## Rationale
The orchestrator spawns agents as subprocesses and needs to propagate real-time events (tool use, text output, lifecycle changes) to optional consumers (TUI store, tests, logging) without coupling layers together. Callbacks in options interfaces let each layer opt in to the events it cares about, while `makeAgentCallbacks` wraps them with cross-cutting concerns like logging.

## Examples

### Example 1: AgentOptions — lowest-level subprocess callbacks
**File**: `src/cli.ts:19`
```typescript
export interface AgentOptions {
  model?: 'opus' | 'sonnet' | 'haiku';
  timeout?: number;
  tools?: string[];
  workingDirectory?: string;
  onToolUse?: (event: ToolUseEvent) => void;
  onToolResult?: (event: ToolResultEvent) => void;
  onText?: (text: string) => void;
}

export async function runAgent(prompt: string, options?: AgentOptions): Promise<AgentResult>
```

### Example 2: OrchestratorOptions — full lifecycle callbacks
**File**: `src/orchestrator.ts:13`
```typescript
export interface OrchestratorOptions {
  projectDir: string;
  maxFixAttempts?: number;
  // TUI lifecycle callbacks (Phase 6 — additive only)
  onStateAssessed?: (state: ProjectState) => void;
  onActionStart?: (action: Action) => void;
  onToolUse?: (event: ToolUseEvent) => void;
  onToolResult?: (event: ToolResultEvent) => void;
  onText?: (text: string) => void;
  onActionComplete?: (action: Action, result: { success: boolean; durationMs: number }) => void;
  onComplete?: () => void;
}
```

### Example 3: makeAgentCallbacks — middleware wrapping callbacks with logging
**File**: `src/orchestrator.ts:437`
```typescript
function makeAgentCallbacks(
  agentSrc: string,
  onToolUseCb?: (event: ToolUseEvent) => void,
  onToolResultCb?: (event: ToolResultEvent) => void,
  onTextCb?: (text: string) => void,
): { onToolUse: ...; onToolResult: ...; onText: ... } {
  return {
    onToolUse: (event) => {
      log('info', agentSrc, formatToolUse(event), { tool: event.tool, input: event.input });
      onToolUseCb?.(event);   // ← forward to caller after logging
    },
    onToolResult: (event) => {
      log(event.error ? 'warn' : 'debug', agentSrc, formatToolResult(event));
      onToolResultCb?.(event);
    },
    onText: (text) => {
      log('info', agentSrc, text);
      onTextCb?.(text);
    },
  };
}
```

### Example 4: StoreActions — the consuming end of the pipeline
**File**: `src/tui/store.ts:37`
```typescript
export interface StoreActions {
  setState: (state: ProjectState) => void;
  startAction: (action: { type: string; phase: string }, model: string) => void;
  addToolUse: (event: ToolUseEvent) => void;    // ← receives onToolUse events
  addToolResult: (event: ToolResultEvent) => void;
  addText: (text: string) => void;
  finishAction: () => void;
  complete: () => void;
  tick: (elapsedMs: number) => void;
}
```

## When to Use
- When a function spawns a subprocess or drives a long-running operation that emits events
- When consumers at different layers need to react to the same events (log to disk AND update UI)
- When the function must work correctly even if no callbacks are provided (all optional)

## When NOT to Use
- For simple synchronous functions — just return a value
- When there is only one consumer — pass a direct parameter instead

## Common Violations
- Making callbacks required — they should always be optional (`cb?:`), because callers without a TUI still need to run without errors
- Skipping the `makeAgentCallbacks` wrapper — callbacks should be wrapped so logging always happens regardless of whether a TUI is attached
- Bypassing the options interface and adding positional callback parameters — keep callbacks in the options object so the signature stays stable as new callbacks are added
