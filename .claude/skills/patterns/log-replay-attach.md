# Pattern: Log Replay Attach

Reconstruct TUI store state by replaying an NDJSON log file, then tail new entries via a polling interval.

## Rationale

The `attach` command must display the current orchestrator state without re-running it. Rather than maintaining a persistent socket or IPC channel, it reads the log file that the orchestrator writes incrementally. `replayLog()` seeds the store from existing content; a `setInterval` polling loop appends new entries as the orchestrator writes them. This leverages the existing `log-dual-output` infrastructure with no new protocols.

## Examples

### Example 1: Replay existing log into store
**File**: `src/tui/replay.ts:19-49`
```typescript
export function replayLog(logPath: string, actions: StoreActions): ReplayResult {
  const content = readFileSync(logPath, 'utf-8');
  const byteOffset = Buffer.byteLength(content, 'utf-8');
  const lines = content.split('\n');
  for (const line of lines) {
    const entry = parseLogLine(line);
    if (entry === null) continue;
    feedLogEntry(entry, actions);
    entryCount++;
  }
  if (firstTs !== null && lastTs !== null) {
    actions.tick(new Date(lastTs).getTime() - new Date(firstTs).getTime());
  }
  return { byteOffset, entryCount };
}
```

### Example 2: Feed a single log entry to store actions
**File**: `src/tui/replay.ts:55-61`
```typescript
export function feedLogEntry(entry: LogEntry, actions: StoreActions): void {
  if (entry.src === 'orchestrator') {
    handleOrchestratorEntry(entry, actions);
  } else if (entry.src.startsWith('agent:')) {
    handleAgentEntry(entry, actions);
  }
}
```

### Example 3: Polling loop in attachCommand
**File**: `src/index.ts:397-431`
```typescript
// Seed from existing content
const { byteOffset } = replayLog(logPath, actions);
let offset = byteOffset;

// createLineBuffer accumulates partial reads; onLine callback feeds entries
const lineBuffer = createLineBuffer((line) => {
  const entry = parseLogLine(line);
  if (entry) feedLogEntry(entry, actions);
});

const pollLog = (): void => {
  const fd = openSync(logPath, 'r');
  const stat = fstatSync(fd);
  if (stat.size <= offset) return;
  const buf = Buffer.alloc(stat.size - offset);
  readSync(fd, buf, 0, buf.length, offset);
  offset = stat.size;
  lineBuffer.push(buf.toString('utf-8'));
  closeSync(fd);
};

const pollInterval = setInterval(pollLog, 200);
```

## When to Use

- Displaying orchestrator output for a background process without modifying it
- Restoring TUI state from a partially-written log at startup
- Any read-only "follow" of an NDJSON log file

## When NOT to Use

- When the orchestrator runs in the same process (use direct `StoreActions` callbacks instead)
- When log files are absent (validate existence before calling `replayLog`)

## Common Violations

- Forgetting to seed `offset` from `byteOffset` — causes the poll loop to re-read already-replayed content
- Parsing the log outside `feedLogEntry` — duplicates the `src`-based dispatch logic
- Using `readFile` in the poll loop instead of offset-based `readSync` — re-reads the entire file every 200ms
