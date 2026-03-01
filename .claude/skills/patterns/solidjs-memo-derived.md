# Pattern: SolidJS Memo for Derived Component State

TUI components use `createMemo()` to derive display values from reactive store props. Memos recompute automatically when their reactive dependencies change and are invoked as functions in JSX.

## Rationale

SolidJS's reactive system tracks reads inside `createMemo()` and re-evaluates only when dependencies change. This keeps expensive derivations (phase progress scanning, log windowing, layout calculations) efficient without manual dependency arrays. It also separates display logic from JSX markup, making components easier to test and read.

## Examples

### Example 1: Multiple memos for header display values
**File**: `src/tui/header.tsx:20-43`
```tsx
const phaseProgress = createMemo(() => {
  const phases = props.store.phases;
  const total = props.store.totalPhases;
  if (total === 0) return '';
  const activeIdx = phases.findIndex((p: PhaseView) => p.status === 'active');
  const current = activeIdx >= 0 ? activeIdx + 1 : total;
  return `Phase ${current}/${total}`;
});

const spinnerChar = createMemo(() => {
  if (props.store.activeAgent === null) return '';
  const idx = Math.floor(props.store.elapsedMs / 80) % SPINNER_FRAMES.length;
  return `  ${SPINNER_FRAMES[idx]!}`;
});

const elapsed = createMemo(() => {
  if (props.store.startedAt === null) return '';
  return `  ${formatElapsed(props.store.elapsedMs)}`;
});

// In JSX — invoked as functions:
<text content={`  ${phaseProgress()}`} />
<text content={spinnerChar()} fg={fg(TUI_COLORS.warning)} />
<text content={elapsed()} fg={fg(TUI_COLORS.muted)} />
```

### Example 2: Memo for filtered/windowed log entries
**File**: `src/tui/log.tsx:64-82`
```tsx
const visibleEntries = createMemo(() => {
  let entries = props.store.logEntries;

  if (!props.showDetail) {
    entries = entries.filter((e: StoreLogEntry) => e.kind !== 'tool_result');
  }

  const total = entries.length;
  const height = props.visibleHeight;
  const offset = props.scrollOffset;

  if (total <= height) return entries;

  const end = total - offset;
  const start = Math.max(0, end - height);
  return entries.slice(start, end > total ? total : end);
});

// In JSX:
<For each={visibleEntries()}>
  {(entry) => { /* render entry */ }}
</For>
```

### Example 3: Layout memos derived from terminal dimensions
**File**: `src/tui/app.tsx:55-68`
```tsx
const sidebarVisible = createMemo(() => props.terminalWidth >= MIN_SIDEBAR_WIDTH);

const logHeight = createMemo(() => {
  return Math.max(1, props.terminalHeight - HEADER_HEIGHT - FOOTER_HEIGHT);
});

const filteredEntryCount = createMemo(() => {
  if (showDetail()) return props.store.logEntries.length;
  return props.store.logEntries.filter((e: StoreLogEntry) => e.kind !== 'tool_result').length;
});
```

## When to Use

- Any derived display value that depends on `props.store.*` fields
- Computations involving iteration or filtering over reactive arrays
- Layout calculations that depend on reactive terminal dimensions or signal values
- Spinner frame selection based on `elapsedMs` (time-driven animation)

## When NOT to Use

- One-time computed values that don't depend on reactive state — compute inline or as a `const` outside the component
- Side-effectful operations — `createMemo()` should be pure with no observable side effects
- Values that depend only on non-reactive props (static strings, module-level constants)

## Common Violations

- Computing derived values inline in JSX without `createMemo()` — re-executes on every render rather than only when dependencies change
- Putting side effects (setStore calls, DOM mutations) inside `createMemo()` — SolidJS may run memos multiple times in development
- Forgetting to call the memo as a function (`phaseProgress` vs `phaseProgress()`) in JSX — returns the signal object instead of the value
