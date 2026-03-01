# Pattern: OpenTUI Native Test Stub

Enable `captureCharFrame()` in Vitest by stubbing `bun:ffi` as a Vite module alias and patching `TextBuffer`/`OptimizedBuffer` prototypes to route rendered text through JS-side accumulators.

## Rationale

`@opentui/core` renders through native FFI: `TextBuffer.setStyledText` writes to a native buffer, and `OptimizedBuffer.getRealCharBytes` reads raw bytes back. Under Vitest (Node.js), the native layer is a no-op stub that returns null bytes, making `captureCharFrame()` return empty strings. Two complementary stubs fix this:

1. **`bun-ffi-stub.ts`** — a Vite `resolveId` alias for `bun:ffi` that returns truthy pointer placeholders so `@opentui/core` loads without errors.
2. **`opentui-test-patch.ts`** — prototype patches that intercept `setStyledText` to accumulate text chunks in JS memory, then override `getRealCharBytes` to return that text, making `captureCharFrame()` work.

## Examples

### Example 1: bun:ffi module stub with truthy pointer stubs
**File**: `tests/helpers/bun-ffi-stub.ts:1-28`
```typescript
export function dlopen(_libPath: string, _symbols: Record<string, unknown>) {
  return {
    symbols: new Proxy({} as Record<string, (...args: unknown[]) => unknown>, {
      get(_target, _prop) {
        return () => 1; // truthy — pointer checks like `if (!ptr)` pass
      },
    }),
  };
}

export class JSCallback {
  ptr = 1; // non-zero so `if (!logCallback.ptr)` passes in @opentui/core
  close() {}
}
```

### Example 2: TextBuffer prototype patch accumulates chunks
**File**: `tests/helpers/opentui-test-patch.ts:19-30`
```typescript
const _capturedChunks: string[] = [];
const _origSetStyledText = (TextBuffer.prototype as any).setStyledText;
(TextBuffer.prototype as any).setStyledText = function (text: any): void {
  if (text?.chunks) {
    for (const chunk of text.chunks as Array<{ text: string }>) {
      if (typeof chunk.text === 'string' && chunk.text.length > 0) {
        _capturedChunks.push(chunk.text);
      }
    }
  }
  return _origSetStyledText.call(this, text);
};
```

### Example 3: OptimizedBuffer patch returns accumulated text to captureCharFrame
**File**: `tests/helpers/opentui-test-patch.ts:37-41`
```typescript
(OptimizedBuffer.prototype as any).getRealCharBytes = function (_addLineBreaks?: boolean): Uint8Array {
  const text = _capturedChunks.join(' ');
  _capturedChunks.length = 0;  // reset accumulator
  return new TextEncoder().encode(text);
};
```

### Example 4: Vitest config wires both stubs
**File**: `vitest.config.ts` (Vite plugin + setupFiles)
```typescript
// bun:ffi alias via resolveId plugin
plugins: [{
  name: 'bun-ffi-stub',
  resolveId(id: string) {
    if (id === 'bun:ffi') return bunFfiStubPath;
  }
}],
// Prototype patches run before any test
setupFiles: ['tests/helpers/opentui-test-patch.ts'],
```

## When to Use

- Any component test that calls `captureCharFrame()` via the `ink-render-helper-test` pattern
- When adding new TUI components that need Vitest unit tests

## When NOT to Use

- Production code — these are test-only stubs
- Integration tests that spawn the full TUI in a real process

## Common Violations

- Forgetting that `_capturedChunks` is a global accumulator — if two `captureCharFrame()` calls overlap, chunks may bleed between frames. Always call `captureCharFrame()` once per `renderOnce()`.
- Importing `opentui-test-patch.ts` in individual test files — it runs globally via `setupFiles`, importing it again resets prototypes and may shadow the accumulator.
