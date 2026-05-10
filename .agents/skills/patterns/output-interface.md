# Pattern: Output Interface

All CLI command output goes through a typed `Output` handle created by `setupOutput(args)`. Three concrete modes (tty/plain/json) share a single interface, so commands need no special-casing for agent or non-interactive contexts.

## Rationale

Commands write to `out.info()`, `out.success()`, `out.error()`, `out.progress()`, and `out.json()` — never to `process.stdout.write` directly. `pickMode()` selects the implementation at startup based on `--json` flag and `isTTY` detection. The tty implementation uses clack spinners and ANSI colors; plain writes clean text lines; json emits structured event objects. This replaces the old `runAgentMode()`/`runInteractiveMode()` explicit split — the mode is resolved once, transparently.

## Examples

### Example 1: Output interface definition
**File**: `packages/core/src/output/types.ts:19`
```typescript
export interface Output {
  readonly mode: OutputMode;  // "tty" | "plain" | "json"

  info(message: string): void;
  warn(message: string, hint?: string): void;
  error(message: string, hint?: string): void;
  success(message: string): void;
  block(lines: string[], opts?: { stream?: "stdout" | "stderr" }): void;

  json<T>(event: T): void;

  progress(label: string): Progress;

  raw(text: string): void;
}
```

### Example 2: Mode selection — pickMode()
**File**: `packages/core/src/output/pick.ts:3`
```typescript
export function pickMode(opts?: OutputOptions): OutputMode {
  if (opts?.json === true) return "json";
  const isTTY = opts?.isTTY !== undefined ? opts.isTTY : process.stdout.isTTY === true;
  return isTTY ? "tty" : "plain";
}
```
- `--json` → `"json"` mode: every event is a JSON line, `info()`/`success()` are no-ops
- TTY stdout → `"tty"` mode: clack spinners, ANSI colors
- piped stdout (CI, agent, `--quiet`) → `"plain"` mode: clean text lines

### Example 3: setupOutput — universal command entry point
**File**: `packages/cli/src/ui/setup.ts:8`
```typescript
export function setupOutput(args: OutputArgs): Output {
  return createOutput({
    json: args.json ?? false,
    quiet: args.quiet ?? false,
  });
}
```
Every command's `run({ args })` handler starts with:
```typescript
const out = setupOutput(args);
```

### Example 4: Command using the Output handle
**File**: `packages/cli/src/commands/install/shared.ts:66`
```typescript
export async function setupInstallContext(args, rawArgs): Promise<InstallContext> {
  const out = setupOutput(args);   // ← single call, mode resolved automatically
  // ...
  const { config, policy } = await loadPolicyOrExit({ ... });
  // ...
  return { out, config, policy, scope, projectRoot, also, runSemantic, agent, verbose };
}
```
Later in the same command:
```typescript
out.success(`Installed ${name} → ${installDir}`);
out.error("Source not recognized", "Run `skilltap find` to search the registry");
const p = out.progress("Cloning...");
p.update("Scanning...");
p.succeed("Done");
```

### Example 5: JSON mode — structured event output
**File**: `packages/cli/src/output/json.ts`
```typescript
// In json mode, every semantic event calls out.json(event)
out.json<InstallEvent>({
  type: "install",
  name: record.name,
  scope,
  path: installDir,
  ref: record.ref ?? null,
});
// In tty/plain mode, out.json() is a no-op — use out.success() for human output
```

### Example 6: Progress handle
**File**: `packages/core/src/output/types.ts:11`
```typescript
export interface Progress {
  update(message: string): void;
  succeed(message?: string): void;
  fail(message?: string): void;
  pause(): void;
  resume(): void;
}
```
In tty mode: wraps a clack spinner. In plain mode: writes a "..." prefix line then "done" on succeed. In json mode: emits `progress:start`, `progress:update`, `progress:done`/`progress:fail` events.

## When to Use

- Every CLI command: call `setupOutput(args)` at the top of `run()` or `setupXxxContext()`
- Pass `out` to core functions that accept `out?: Output` for progress reporting
- Use `out.json(event)` alongside `out.success()` — json mode ignores success, tty/plain ignore json
- Core defines the `Output` interface; CLI owns the three concrete implementations

## When NOT to Use

- `process.stdout.write` / `console.log` directly in command handlers — always use `out`
- `out.json()` for non-structured text — it's for machine-parseable event objects
- `log.*` from `@clack/prompts` in command files — use `out.info()` / `out.warn()` instead (clack is for interactive prompts only)

## Common Violations

- Writing to stdout/stderr directly from a command instead of through `out`
- Calling `out.json()` only and forgetting `out.success()` — human output disappears in tty/plain mode
- Forgetting to pass `out` to core functions that accept it — progress disappears in all modes
- Checking `out.mode === "json"` to branch behavior — pass the right output calls instead; modes handle themselves
