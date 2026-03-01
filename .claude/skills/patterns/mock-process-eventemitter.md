# Pattern: Mock Process EventEmitter

A `ChildProcess` mock is created by casting an `EventEmitter` instance, then attaching `stdout`/`stderr`/`stdin` as child `EventEmitter` instances.

## Rationale

`ChildProcess` is not directly constructable, but it extends `EventEmitter` and exposes `stdout`/`stderr` as streams. Tests cast a plain `EventEmitter` to `ChildProcess` and assign sub-emitters for I/O channels, enabling test code to emit data/close events that drive the subprocess parsing logic in `cli.ts` without spawning a real process.

## Examples

### Example 1: Shared factory with optional auto-complete callback
**File**: `tests/helpers/factories.ts:42`
```typescript
export function createMockProcess(onStdout?: (proc: ChildProcess) => void): ChildProcess {
  const proc = new EventEmitter() as ChildProcess;
  proc.stdout = new EventEmitter() as ChildProcess['stdout'];
  proc.stderr = new EventEmitter() as ChildProcess['stderr'];
  proc.stdin = new EventEmitter() as ChildProcess['stdin'];
  proc.kill = vi.fn().mockReturnValue(true);
  proc.killed = false;
  proc.pid = 1234;

  // Auto-complete the process on nextTick (only when callback provided)
  if (onStdout !== undefined) {
    process.nextTick(() => {
      onStdout(proc);
      proc.stdout!.emit('data', Buffer.from('{"type":"result","result":"done"}\n'));
      proc.emit('close', 0);
    });
  }

  return proc;
}
```

### Example 2: Emitting NDJSON stream events in unit tests
**File**: `tests/unit/cli.test.ts:35`
```typescript
const proc = createMockProcess();
mockSpawn.mockReturnValue(proc);

const promise = runAgent('test prompt');

// Wait for spawn to be called (after exec resolves via nextTick)
await vi.waitFor(() => {
  expect(mockSpawn).toHaveBeenCalled();
});

proc.stdout!.emit('data', Buffer.from(
  '{"type":"result","result":{"content":[{"type":"text","text":"Hello world"}]}}\n'
));
proc.emit('close', 0);

const result = await promise;
expect(result.success).toBe(true);
```

### Example 3: Using auto-complete callback for integration agent simulations
**File**: `tests/integration/orchestrator.test.ts:70`
```typescript
createMockProcess(async (proc) => {
  // Simulate agent writing files during execution
  await writeFile(join(phaseDir, 'VISION.md'), 'vision content');
  proc.stdout!.emit('data', Buffer.from(
    '{"type":"result","result":{"content":[{"type":"text","text":"done"}]}}\n'
  ));
});
```

## When to Use

- Any test that exercises code which calls `spawn()` and reads from `proc.stdout`
- Integration tests that need to simulate a complete agent run with specific NDJSON payloads
- Tests verifying timeout/kill behavior by controlling when `close` fires

## When NOT to Use

- Testing code that uses `exec()` — use `mockExecSuccess()`/`mockExecFailure()` helpers from `tests/helpers/setup.ts` instead
- End-to-end tests that need to verify actual Claude CLI output

## Common Violations

- Emitting events synchronously before `await vi.waitFor(() => expect(mockSpawn).toHaveBeenCalled())` — the `spawn` call is async, so events fired before the listener is attached are silently dropped
- Forgetting to emit `close` — `runAgent` waits for the close event to resolve the promise
