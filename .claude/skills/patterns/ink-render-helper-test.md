# Pattern: Ink Component Render Helper Test

Each Ink/SolidJS component test file defines a local async `render*` helper that wraps `testRender()` from `@opentui/solid`. The helper constructs the store from factory overrides, renders the component, and returns a captured character frame for string assertions.

## Rationale

`testRender()` returns a renderer object that must be awaited; capturing the frame requires calling `renderOnce()` first. By wrapping this boilerplate in a local helper, test cases stay concise and consistent. Each helper is local (not shared) because components have different prop shapes, but all helpers follow the same structure.

## Examples

### Example 1: Header render helper
**File**: `tests/unit/header.test.tsx:11-18`
```typescript
async function renderHeader(overrides: Partial<TuiStore> = {}, projectDir = '/home/user/my-project') {
  const store = makeTuiStore(overrides);
  const { captureCharFrame, renderOnce } = await testRender(
    () => <Header store={store} projectDir={projectDir} />
  );
  await renderOnce();
  return { frame: captureCharFrame(), store };
}
```

### Example 2: Log render helper with scroll options
**File**: `tests/unit/log.test.tsx:10-27`
```typescript
async function renderLog(
  storeOverrides: Partial<TuiStore> = {},
  opts: { scrollOffset?: number; showDetail?: boolean; visibleHeight?: number } = {},
) {
  const store = makeTuiStore(storeOverrides);
  const { captureCharFrame, renderOnce } = await testRender(
    () => (
      <AgentLog
        store={store}
        scrollOffset={opts.scrollOffset ?? 0}
        showDetail={opts.showDetail ?? false}
        visibleHeight={opts.visibleHeight ?? 10}
      />
    ),
  );
  await renderOnce();
  return { frame: captureCharFrame(), store };
}
```

### Example 3: App render helper returning actions + keyInput
**File**: `tests/unit/app.test.tsx:16-40`
```typescript
async function renderApp(storeOverrides: Partial<TuiStore> = {}) {
  const store = makeTuiStore(storeOverrides);
  const actions = createMockStoreActions();
  const keyInput = new EventEmitter();
  const onQuit = vi.fn();
  const { captureCharFrame, renderOnce, destroy } = await testRender(
    () => (
      <App
        store={store}
        actions={actions}
        projectDir="/home/user/my-project"
        terminalWidth={80}
        terminalHeight={24}
        keyInput={keyInput as unknown as KeyInput}
        onQuit={onQuit}
      />
    ),
  );
  await renderOnce();
  return { frame: captureCharFrame(), store, actions, keyInput, onQuit, renderOnce, destroy };
}
```

### Example 4: Using the helper in test assertions
**File**: `tests/unit/header.test.tsx:22-30`
```typescript
it('idle state — no agent, no spinner', async () => {
  const { frame } = await renderHeader({ activeAgent: null, startedAt: null });
  expect(frame).toContain('my-project');
  for (const ch of SPINNER_FRAMES) {
    expect(frame).not.toContain(ch);
  }
});
```

## When to Use

- Any Ink/SolidJS component that needs rendering in tests
- When testing component output via character frame string assertions
- When multiple tests in a file share the same component + default prop setup

## When NOT to Use

- Unit tests for pure functions (format helpers, view transforms) — no rendering needed
- Store unit tests — test action functions directly without rendering

## Common Violations

- Forgetting `await renderOnce()` before calling `captureCharFrame()` — returns an empty/initial frame
- Sharing a single render helper across multiple component test files — each component has unique prop shapes, keep helpers local
- Using snapshot tests instead of `toContain()` string assertions — brittle against terminal width/padding changes
