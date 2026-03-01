# Pattern: Solid Reactive Store

TUI state is managed via a Solid.js `createStore` that returns a `{ store, actions }` pair — the store is the reactive signal graph, actions are the only mutation surface.

## Rationale
The TUI needs fine-grained reactivity (only changed fields re-render) and a bounded, auditable mutation surface. Solid's `createStore`/`setStore` provides path-based partial updates so only the changed fields trigger renders. Returning `{ store, actions }` from a factory function encapsulates the reactive state and exposes a typed action API without leaking `setStore` to callers.

## Examples

### Example 1: createTuiStore factory — creates store + actions
**File**: `src/tui/store.ts:54`
```typescript
export function createTuiStore(): { store: TuiStore; actions: StoreActions } {
  const [store, setStore] = createStore<TuiStore>({
    phases: [],
    currentPhaseName: null,
    totalPhases: 0,
    activeAgent: null,
    logEntries: [],
    buildPasses: false,
    isComplete: false,
    toolCount: 0,
    // ... all fields initialised
  });

  const actions: StoreActions = {
    setState(state: ProjectState): void {
      setStore({                           // ← partial object update
        phases: toPhaseViews(state),
        currentPhaseName: state.currentPhase?.name ?? null,
        totalPhases: state.phases.length,
        buildPasses: state.buildPasses,
        testsPassing: state.testsPassing,
      });
    },
    // ...
  };

  return { store, actions };  // ← caller never sees setStore
}
```

### Example 2: Ring-buffer log entries — capped array update via setStore
**File**: `src/tui/store.ts:72`
```typescript
const MAX_LOG_ENTRIES = 1000;

function appendLogEntry(entry: StoreLogEntry): void {
  const current = store.logEntries;
  if (current.length >= MAX_LOG_ENTRIES) {
    // Drop oldest entry; Solid detects the array identity change
    setStore('logEntries', [...current.slice(current.length - MAX_LOG_ENTRIES + 1), entry]);
  } else {
    setStore('logEntries', [...current, entry]);
  }
}
```

### Example 3: StoreActions interface — typed mutation contract
**File**: `src/tui/store.ts:37`
```typescript
export interface StoreActions {
  setState: (state: ProjectState) => void;
  startAction: (action: { type: string; phase: string }, model: string) => void;
  addToolUse: (event: ToolUseEvent) => void;
  addToolResult: (event: ToolResultEvent) => void;
  addText: (text: string) => void;
  finishAction: () => void;
  complete: () => void;
  tick: (elapsedMs: number) => void;
}
```

## When to Use
- When TUI components need reactive state that updates incrementally (not full rerenders)
- When multiple independent consumers receive events and each needs to update its own slice of state
- When the log/event list must be bounded to prevent unbounded memory growth

## When NOT to Use
- For state that doesn't drive a reactive UI — plain objects are simpler
- For state shared only within a single function — local variables are sufficient

## Common Violations
- Exposing `setStore` directly to callers — all mutations must go through `actions` so the mutation surface is auditable
- Replacing the entire `logEntries` array on each append without slicing — without the ring-buffer slice, memory grows without bound during a long orchestration run
- Creating multiple stores for loosely related UI state — one store per TUI context keeps reactivity dependencies co-located
