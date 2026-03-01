# Pattern: Log Dual Output

Structured `LogEntry` objects are serialized as NDJSON to a log file AND formatted as colored, human-readable strings to stdout. A matching parse→format pipeline allows live tailing from other processes.

## Rationale

The orchestrator runs as a background daemon; the TUI `attach` command must tail its output without re-spawning it. Writing JSON to a file and ANSI-formatted text to stdout simultaneously satisfies both the attach-mode consumer (structured data for re-formatting) and the foreground operator (readable output). `parseLogLine` validates the shape of entries read back from disk, and `formatLogEntry` produces the same human display regardless of which process calls it.

## Examples

### Example 1: Writing — JSON to file, formatted to stdout
**File**: `src/log.ts:41`
```typescript
export function log(level: LogLevel, src: string, msg: string, data?: unknown): void {
  const entry: LogEntry = {
    ts: new Date().toISOString(),
    level,
    src,
    msg,
    ...(data !== undefined && { data }),
  };

  // Write JSON to log file if initialized
  if (logFilePath) {
    appendFileSync(logFilePath, JSON.stringify(entry) + '\n');
  }

  // Suppress debug from stdout unless LOG_LEVEL=debug
  if (level === 'debug' && process.env.LOG_LEVEL !== 'debug') return;

  process.stdout.write(formatLogEntry(entry) + '\n');
}
```

### Example 2: Parsing — round-trip from file back to LogEntry
**File**: `src/log.ts:66`
```typescript
export function parseLogLine(line: string): LogEntry | null {
  const trimmed = line.trim();
  if (!trimmed) return null;

  try {
    const obj = JSON.parse(trimmed) as Record<string, unknown>;
    if (
      typeof obj.ts !== 'string' ||
      typeof obj.level !== 'string' ||
      typeof obj.src !== 'string' ||
      typeof obj.msg !== 'string'
    ) {
      return null;
    }
    return { ts: obj.ts, level: obj.level as LogLevel, src: obj.src, msg: obj.msg,
      ...(obj.data !== undefined && { data: obj.data }) };
  } catch {
    return null;  // Malformed line — silently skip
  }
}
```

### Example 3: Tailing — byte-offset polling loop using the parse→format pipeline
**File**: `src/index.ts:354`
```typescript
const pollLog = (): void => {
  let fd: number;
  try {
    fd = openSync(logPath, 'r');
  } catch {
    return;
  }
  try {
    const stat = fstatSync(fd);
    if (stat.size <= offset) return;

    const bytesToRead = stat.size - offset;
    const buf = Buffer.alloc(bytesToRead);
    readSync(fd, buf, 0, bytesToRead, offset);
    offset = stat.size;

    lineBuffer += buf.toString('utf-8');

    let newlineIndex: number;
    while ((newlineIndex = lineBuffer.indexOf('\n')) !== -1) {
      const rawLine = lineBuffer.slice(0, newlineIndex);
      lineBuffer = lineBuffer.slice(newlineIndex + 1);

      const entry = parseLogLine(rawLine);      // parse from disk
      if (entry) {
        process.stdout.write(formatLogEntry(entry) + '\n');  // same format as writer
      }
    }
  } finally {
    closeSync(fd);
  }
};

const pollInterval = setInterval(pollLog, 200);
```

## When to Use

- Any new log emission in the orchestrator — always use `log()`, never `process.stdout.write` directly
- Implementing a new `attach`-style tail consumer — use `parseLogLine` → `formatLogEntry`
- Adding a new log source tag — add a color mapping in `colorize()` in `log.ts:111`

## When NOT to Use

- Fatal CLI errors before the daemon starts — use `process.stderr.write` + `process.exit(1)` directly (no log file yet)
- Test output or user-facing formatted tables — use `process.stdout.write` directly

## Common Violations

- Calling `process.stdout.write` in orchestrator code instead of `log()` — bypasses file logging and loses structured data
- Parsing log lines with custom JSON parsing instead of `parseLogLine` — skips shape validation and returns malformed entries
