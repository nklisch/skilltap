# Design: Phase 41 — Output Mode Abstraction

## Overview

Phase 40 consolidated all CLI writes through `packages/cli/src/ui/format.ts` helpers
(`successLine`, `errorLine`, `infoLine`, `jsonLine`, `securityBlock`). Phase 41 builds an
**`Output` interface** over those helpers with three modes — `tty`, `plain`, `json` —
selected per-command at entry. After Phase 41:

- Every CLI command constructs an `Output` at entry and writes through it.
- Mode resolves from `--json` flag, TTY detection, or explicit override.
- JSON mode emits **newline-delimited structured events** (NDJSON), not pretty-printed blobs.
- Plain mode (piped stdin/stdout) strips colors and disables spinners; output stays
  scriptable.
- Per-command Zod event schemas validate JSON output (initial coverage on the highest-traffic
  commands; others use `z.unknown()` placeholders for follow-up tightening).
- `try.ts`'s 18 raw `process.stdout.write` calls and `index.ts`'s 11 startup `process.stderr.write`
  calls all migrate to `Output` methods.

This phase does **not** rewrite `format.ts`. The TTY-mode adapter wraps it. format.ts becomes
the implementation backing the TTY mode; Phase 41 is the **port**, format.ts is one **adapter**.

## Scope: what's in vs. out

**In scope:**
- `Output` interface in `packages/core/src/output/`.
- `pickMode()` resolver.
- Three adapters in `packages/cli/src/output/`: tty (wraps format.ts + clack), plain, json.
- Migrate every CLI command to construct an `Output` and write through it. ~57 files touched.
- Zod schemas for the 16 commands that register `--json` today. Initial events; commands with
  multi-step JSON streams get full discriminated unions.
- Replace clack `spinner()` usage (10 files) with `out.progress()` returning a `Progress` handle.
- Convert `try.ts` raw stdout writes and `index.ts` startup messages to `Output` calls.

**Out of scope:**
- Rewriting any output text (formatting stays identical in TTY mode).
- Changing what each command's JSON output **contains** — the existing shapes survive, just get
  a Zod schema and an `Output.json()` call site.
- Phase 44's TUI (Ink) — that builds atop the Output interface but is its own phase.

## Acceptance criteria (project-wide)

- `grep -rn "process\.stdout\|process\.stderr" packages/cli/src/ --include="*.ts" | grep -v ".test.ts" | grep -v "/output/" | grep -v "/completions/"` returns nothing or only documented escape hatches (the bash-completion protocol writer in `completions/dynamic.ts` is the only legitimate `console.log` site).
- `grep -rn "successLine\|errorLine\|infoLine\|warnLine\|jsonLine\|securityBlock" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"` returns nothing — all replaced by `out.*` methods.
- Every command file imports `createOutput` from `cli/src/output` and constructs `out` at entry.
- `bun test` passes (suite count may shift slightly; net same coverage).
- `skilltap doctor --json` and `skilltap status --json` produce NDJSON validatable against the per-command schemas.
- `skilltap install <repo>` piped (no TTY): plain text, no spinner, no ANSI codes.
- `skilltap install <repo>` interactive: clack spinner + colors as today.

## Architectural options considered

### Option A — Imperative module with hidden mode state
Single `output` module with module-level state. `setMode()` at entry; functions like `output.success(msg)`. Pro: minimal disruption. Con: hidden global, hard to test concurrent commands, no clear DI seam.

### Option B — Instance per command (chosen)
`createOutput(opts) → Output`. Each command builds an `Output` at entry and threads it. Methods on the instance: `out.success(...)`. Pro: no global state, mockable in tests, explicit data flow, supports nested invocations (e.g., sync calling install). Con: every command function signature accepts `Output`; that's mechanical but real.

### Option C — Hybrid (default + override)
Module-level default with `withOutput(out, fn)` override for tests. Pro: minimum migration effort. Con: still hides global state from readers; the threaded-instance approach is barely more work and more honest.

**Choice: Option B.** Skilltap already threads policy + callbacks through every command. Adding `Output` is one more parameter, mechanically. Tests construct an in-memory `CaptureOutput` and assert on its captured events directly — no subprocess parsing, no ANSI stripping. Concurrent or nested invocations are isolated.

## Module layering — port + adapter

The `Output` **interface** is the port; it lives in `core` so other consumers (future SDK,
test-utils) can target it. The **adapters** (tty, plain, json) live in `cli` so they can depend
on format.ts and `@clack/prompts` without leaking those into core.

```
packages/core/src/output/
├── types.ts       # Output interface, OutputMode, OutputOptions, Progress
├── pick.ts        # pickMode(opts) → OutputMode
└── index.ts       # barrel

packages/cli/src/output/
├── tty.ts         # TTY adapter (wraps format.ts helpers + clack spinner)
├── plain.ts       # Plain-text adapter (no colors, no spinners)
├── json.ts        # NDJSON adapter (one JSON event per line)
├── factory.ts     # createOutput(opts) — dispatches to the right adapter
├── capture.ts     # CaptureOutput for tests (in-memory event buffer)
├── schemas.ts     # Per-command Zod event schemas
└── index.ts       # barrel
```

Core never imports cli. Cli imports `Output`, `OutputMode`, `OutputOptions`, `Progress` types
from core, plus `pickMode`. Test-utils (a sibling package) gains a new export: `CaptureOutput`
constructor for tests that want to assert on events without spawning a subprocess.

## Trickiest unit — designed first

**Unit 6 — `Output.progress()` semantics**. The `progress()` method returns a `Progress`
handle that callers update over time. In TTY mode it must wrap a clack spinner; in plain
mode it should emit a single line per `update()` (or be silent); in JSON mode it must
emit a `progress:*` event per update. The risk: `install.ts` and `update.ts` start a
spinner, stop it for prompts, then restart — the Progress handle must support this
stop/restart pattern without leaking spinner state into JSON mode.

```typescript
export interface Progress {
  /** Updates the in-flight message. TTY: spinner.message(). plain: emits "label: msg". json: { kind: "progress", label, message }. */
  update(message: string): void;
  /** Marks done. TTY: spinner.stop(message, 0). plain: emits final line. json: { kind: "progress:done", label, message }. */
  succeed(message?: string): void;
  /** Marks failed. TTY: spinner.stop(message, 1). plain: emits "error: msg" to stderr. json: { kind: "progress:fail", label, message }. */
  fail(message?: string): void;
  /** Pause without finishing. TTY: spinner.stop() but state preserved. plain/json: no-op. */
  pause(): void;
  /** Resume after pause. TTY: spinner.start(label). plain/json: no-op. */
  resume(): void;
}
```

The `pause()` / `resume()` pair handles the install.ts/update.ts pattern. Core functions
that take `onProgress` callbacks map naturally onto `out.progress()`.

## Implementation Units

### Unit 1 — Output interface + types

**File**: `packages/core/src/output/types.ts`

```typescript
export type OutputMode = "tty" | "plain" | "json";

export interface OutputOptions {
  /** --json flag from CLI args. */
  json?: boolean;
  /** --quiet flag from CLI args. */
  quiet?: boolean;
  /** Override TTY detection (testing). */
  isTTY?: boolean;
  /** Override stdout/stderr destinations (testing — defaults to process.std*). */
  stdout?: NodeJS.WritableStream;
  stderr?: NodeJS.WritableStream;
}

export interface Progress {
  update(message: string): void;
  succeed(message?: string): void;
  fail(message?: string): void;
  pause(): void;
  resume(): void;
}

export interface Output {
  readonly mode: OutputMode;

  // Human-facing — no-op in json mode.
  info(message: string): void;
  warn(message: string, hint?: string): void;
  success(message: string): void;
  block(lines: string[], opts?: { stream?: "stdout" | "stderr" }): void;

  // Errors — always emitted. In json mode emits an `{ kind: "error", message, hint? }` event.
  error(message: string, hint?: string): void;

  // Structured events. In tty/plain modes, no-op. In json mode, NDJSON to stdout.
  json<T>(event: T): void;

  // Long-running progress.
  progress(label: string): Progress;

  // Escape hatch — always writes raw to stdout. Used by completions/dynamic.ts.
  raw(text: string): void;
}
```

**Acceptance criteria**:
- [ ] All types exported from `packages/core/src/output/types.ts`.
- [ ] No runtime code (interface-only file).
- [ ] `OutputMode` is a string literal union.

---

### Unit 2 — pickMode + index

**File**: `packages/core/src/output/pick.ts`

```typescript
import type { OutputMode, OutputOptions } from "./types";

/**
 * Resolve output mode from explicit flag, TTY detection, and overrides.
 * Order: explicit json flag wins; otherwise TTY → "tty", else "plain".
 */
export function pickMode(opts?: OutputOptions): OutputMode {
  if (opts?.json === true) return "json";
  const isTTY =
    opts?.isTTY !== undefined ? opts.isTTY : process.stdout.isTTY === true;
  return isTTY ? "tty" : "plain";
}
```

**File**: `packages/core/src/output/index.ts`

```typescript
export type {
  Output,
  OutputMode,
  OutputOptions,
  Progress,
} from "./types";
export { pickMode } from "./pick";
```

Update `packages/core/src/index.ts` to re-export the output module.

**Acceptance criteria**:
- [ ] `pickMode({ json: true })` → `"json"` regardless of TTY.
- [ ] `pickMode({ json: false, isTTY: true })` → `"tty"`.
- [ ] `pickMode({ json: false, isTTY: false })` → `"plain"`.
- [ ] `pickMode()` reads `process.stdout.isTTY` when no override is given.

---

### Unit 3 — TTY adapter

**File**: `packages/cli/src/output/tty.ts`

Wraps existing `format.ts` helpers and `@clack/prompts` spinner.

```typescript
import { spinner } from "@clack/prompts";
import type { Output, OutputOptions, Progress } from "@skilltap/core";
import {
  ansi,
  errorLine,
  infoLine,
  successLine,
} from "../ui/format";

export function createTtyOutput(opts: OutputOptions): Output {
  const quiet = opts.quiet ?? false;
  const stdout = opts.stdout ?? process.stdout;
  const stderr = opts.stderr ?? process.stderr;

  return {
    mode: "tty",
    info(msg) {
      if (quiet) return;
      infoLine(msg);
    },
    warn(msg, hint) {
      stderr.write(`${ansi.yellow("warning")}: ${msg}\n`);
      if (hint) stderr.write(`  ${ansi.dim("hint")}: ${hint}\n`);
    },
    error(msg, hint) {
      errorLine(msg, hint);
    },
    success(msg) {
      if (quiet) return;
      successLine(msg);
    },
    block(lines, opts) {
      const out = (opts?.stream ?? "stderr") === "stdout" ? stdout : stderr;
      out.write(`${lines.join("\n")}\n`);
    },
    json() {
      // no-op in tty mode
    },
    progress(label) {
      return createTtyProgress(label, quiet);
    },
    raw(text) {
      stdout.write(text);
    },
  };
}

function createTtyProgress(label: string, quiet: boolean): Progress {
  if (quiet) return noopProgress();
  const s = spinner();
  s.start(label);
  let active = true;
  return {
    update(msg) {
      if (active) s.message(msg);
    },
    succeed(msg) {
      if (active) {
        s.stop(msg ?? label);
        active = false;
      }
    },
    fail(msg) {
      if (active) {
        s.stop(msg ?? label, 1);
        active = false;
      }
    },
    pause() {
      if (active) {
        s.stop();
        active = false;
      }
    },
    resume() {
      if (!active) {
        s.start(label);
        active = true;
      }
    },
  };
}

function noopProgress(): Progress {
  return {
    update() {},
    succeed() {},
    fail() {},
    pause() {},
    resume() {},
  };
}
```

**Implementation Notes**:
- Imports format.ts directly for color/decoration parity with pre-Phase-41 output.
- `block` defaults to stderr (matches existing securityBlock behavior).
- `quiet` suppresses info/success/progress; errors and warnings still surface.

**Acceptance criteria**:
- [ ] `success("X")` writes "✓ X\n" to stdout (matches existing `successLine`).
- [ ] `error("X", "hint")` writes "error: X\n  hint: hint\n" to stderr (matches existing `errorLine`).
- [ ] `progress("Loading")` returns a handle whose `succeed()` stops the spinner.
- [ ] `pause()` then `resume()` restarts the spinner with the original label.
- [ ] `json()` is a no-op (does not write to any stream).

---

### Unit 4 — Plain adapter

**File**: `packages/cli/src/output/plain.ts`

```typescript
import type { Output, OutputOptions, Progress } from "@skilltap/core";

export function createPlainOutput(opts: OutputOptions): Output {
  const quiet = opts.quiet ?? false;
  const stdout = opts.stdout ?? process.stdout;
  const stderr = opts.stderr ?? process.stderr;

  return {
    mode: "plain",
    info(msg) {
      if (quiet) return;
      stdout.write(`${msg}\n`);
    },
    warn(msg, hint) {
      stderr.write(`warning: ${msg}\n`);
      if (hint) stderr.write(`  hint: ${hint}\n`);
    },
    error(msg, hint) {
      stderr.write(`error: ${msg}\n`);
      if (hint) stderr.write(`  hint: ${hint}\n`);
    },
    success(msg) {
      if (quiet) return;
      stdout.write(`${msg}\n`);
    },
    block(lines, opts) {
      const out = (opts?.stream ?? "stderr") === "stdout" ? stdout : stderr;
      out.write(`${lines.join("\n")}\n`);
    },
    json() {
      // no-op in plain mode
    },
    progress(label) {
      // Single line on start, single line on succeed/fail.
      if (!quiet) stdout.write(`${label}...\n`);
      return {
        update() {},
        succeed(msg) {
          if (!quiet) stdout.write(`${msg ?? label} done\n`);
        },
        fail(msg) {
          stderr.write(`error: ${msg ?? label} failed\n`);
        },
        pause() {},
        resume() {},
      };
    },
    raw(text) {
      stdout.write(text);
    },
  };
}
```

**Acceptance criteria**:
- [ ] No ANSI escape sequences in plain-mode output (verified by regex test).
- [ ] `progress("X")` emits "X...\n" on creation, "done\n" on succeed.
- [ ] `pause()`/`resume()` are no-ops (don't crash, don't emit).

---

### Unit 5 — JSON adapter

**File**: `packages/cli/src/output/json.ts`

```typescript
import type { Output, OutputOptions, Progress } from "@skilltap/core";

export function createJsonOutput(opts: OutputOptions): Output {
  const stdout = opts.stdout ?? process.stdout;
  const stderr = opts.stderr ?? process.stderr;

  return {
    mode: "json",
    info() {
      // no-op — info is human-facing
    },
    warn(msg, hint) {
      stdout.write(`${JSON.stringify({ kind: "warn", message: msg, hint })}\n`);
    },
    error(msg, hint) {
      stdout.write(`${JSON.stringify({ kind: "error", message: msg, hint })}\n`);
    },
    success() {
      // no-op — emit explicit events instead via json()
    },
    block() {
      // no-op — block is human-facing; equivalent JSON event must be emitted via json()
    },
    json(event) {
      stdout.write(`${JSON.stringify(event)}\n`);
    },
    progress(label) {
      stdout.write(`${JSON.stringify({ kind: "progress:start", label })}\n`);
      return {
        update(msg) {
          stdout.write(`${JSON.stringify({ kind: "progress:update", label, message: msg })}\n`);
        },
        succeed(msg) {
          stdout.write(`${JSON.stringify({ kind: "progress:done", label, message: msg })}\n`);
        },
        fail(msg) {
          stdout.write(`${JSON.stringify({ kind: "progress:fail", label, message: msg })}\n`);
        },
        pause() {},
        resume() {},
      };
    },
    raw(text) {
      stdout.write(text);
    },
  };
}
```

**Implementation Notes**:
- All JSON events go to **stdout** (not stderr). One JSON object per line. Final newline included.
- `error()` in JSON mode writes the error event to stdout (still NDJSON), then the calling
  command typically `process.exit(1)` separately.
- `info`/`success`/`block` are no-ops — JSON consumers don't want chatty narration; they want
  the structured events. Commands that want a "summary" event in JSON mode emit it via `out.json(...)`.

**Acceptance criteria**:
- [ ] Every output is valid JSON followed by `\n`.
- [ ] `info`/`success`/`block` produce no output.
- [ ] `error` writes a `{ kind: "error", ... }` event to stdout.
- [ ] `progress` lifecycle emits `progress:start`, `progress:update`, `progress:done` / `progress:fail`.

---

### Unit 6 — Factory + barrel

**File**: `packages/cli/src/output/factory.ts`

```typescript
import { type Output, type OutputOptions, pickMode } from "@skilltap/core";
import { createJsonOutput } from "./json";
import { createPlainOutput } from "./plain";
import { createTtyOutput } from "./tty";

export function createOutput(opts: OutputOptions = {}): Output {
  const mode = pickMode(opts);
  switch (mode) {
    case "tty":
      return createTtyOutput(opts);
    case "plain":
      return createPlainOutput(opts);
    case "json":
      return createJsonOutput(opts);
  }
}
```

**File**: `packages/cli/src/output/index.ts`

```typescript
export { createOutput } from "./factory";
export { CaptureOutput, createCaptureOutput } from "./capture";
// schemas.ts re-exports happen in a separate module
```

**Acceptance criteria**:
- [ ] `createOutput({ json: true })` returns a JSON adapter.
- [ ] `createOutput({ isTTY: true })` returns a TTY adapter.
- [ ] `createOutput({ isTTY: false })` returns a plain adapter.

---

### Unit 7 — CaptureOutput for tests

**File**: `packages/cli/src/output/capture.ts`

```typescript
import type { Output, OutputMode, Progress } from "@skilltap/core";

export type CapturedEvent =
  | { kind: "info"; message: string }
  | { kind: "warn"; message: string; hint?: string }
  | { kind: "error"; message: string; hint?: string }
  | { kind: "success"; message: string }
  | { kind: "block"; lines: string[]; stream: "stdout" | "stderr" }
  | { kind: "json"; event: unknown }
  | { kind: "progress:start"; label: string }
  | { kind: "progress:update"; label: string; message: string }
  | { kind: "progress:done"; label: string; message?: string }
  | { kind: "progress:fail"; label: string; message?: string }
  | { kind: "raw"; text: string };

export interface CaptureOutput extends Output {
  events: CapturedEvent[];
}

export function createCaptureOutput(mode: OutputMode = "plain"): CaptureOutput {
  const events: CapturedEvent[] = [];
  const out: CaptureOutput = {
    events,
    mode,
    info(message) { events.push({ kind: "info", message }); },
    warn(message, hint) { events.push({ kind: "warn", message, hint }); },
    error(message, hint) { events.push({ kind: "error", message, hint }); },
    success(message) { events.push({ kind: "success", message }); },
    block(lines, opts) { events.push({ kind: "block", lines, stream: opts?.stream ?? "stderr" }); },
    json(event) { events.push({ kind: "json", event }); },
    progress(label) {
      events.push({ kind: "progress:start", label });
      return {
        update(message) { events.push({ kind: "progress:update", label, message }); },
        succeed(message) { events.push({ kind: "progress:done", label, message }); },
        fail(message) { events.push({ kind: "progress:fail", label, message }); },
        pause() {},
        resume() {},
      };
    },
    raw(text) { events.push({ kind: "raw", text }); },
  };
  return out;
}
```

**Acceptance criteria**:
- [ ] Calling any `Output` method records an event in `events`.
- [ ] Test code can assert: `expect(out.events).toContainEqual({ kind: "success", message: "Installed X" })`.
- [ ] Mode is configurable so tests can assert mode-specific behavior.

---

### Unit 8 — JSON event schemas (initial)

**File**: `packages/cli/src/output/schemas.ts`

Per-command schemas as discriminated unions. Initial coverage: `install`, `update`, `sync`,
`doctor`, `status`. Other JSON commands get a placeholder schema to be tightened in follow-up.

```typescript
import { z } from "zod/v4";

// Shared event helpers
const ErrorEvent = z.object({
  kind: z.literal("error"),
  message: z.string(),
  hint: z.string().optional(),
});

const ProgressStartEvent = z.object({ kind: z.literal("progress:start"), label: z.string() });
const ProgressUpdateEvent = z.object({ kind: z.literal("progress:update"), label: z.string(), message: z.string() });
const ProgressDoneEvent = z.object({ kind: z.literal("progress:done"), label: z.string(), message: z.string().optional() });
const ProgressFailEvent = z.object({ kind: z.literal("progress:fail"), label: z.string(), message: z.string().optional() });

export const InstallEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("install:start"), source: z.string() }),
  z.object({ kind: z.literal("install:placed"), name: z.string(), path: z.string() }),
  z.object({ kind: z.literal("install:captured"), pluginName: z.string(), skills: z.array(z.string()), mcpServers: z.array(z.string()) }),
  z.object({ kind: z.literal("install:done"), records: z.array(z.string()), pluginName: z.string().optional() }),
  ErrorEvent,
  ProgressStartEvent,
  ProgressUpdateEvent,
  ProgressDoneEvent,
  ProgressFailEvent,
]);

export const UpdateEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("update:start"), name: z.string() }),
  z.object({ kind: z.literal("update:up-to-date"), name: z.string() }),
  z.object({ kind: z.literal("update:updated"), name: z.string(), fromRef: z.string().nullable(), toRef: z.string().nullable() }),
  z.object({ kind: z.literal("update:skipped"), name: z.string(), reason: z.string() }),
  z.object({ kind: z.literal("update:done"), updated: z.array(z.string()), skipped: z.array(z.string()), upToDate: z.array(z.string()) }),
  ErrorEvent,
  ProgressStartEvent,
  ProgressUpdateEvent,
  ProgressDoneEvent,
  ProgressFailEvent,
]);

export const SyncEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("sync:plan"), inSync: z.boolean(), items: z.array(z.unknown()) }),
  z.object({ kind: z.literal("sync:item"), source: z.string(), status: z.enum(["ok", "skipped", "fail"]), error: z.string().optional() }),
  z.object({ kind: z.literal("sync:done"), inSync: z.boolean(), applied: z.number(), skipped: z.number(), failed: z.number() }),
  ErrorEvent,
]);

export const DoctorEventSchema = z.object({
  ok: z.boolean(),
  checks: z.array(
    z.object({
      name: z.string(),
      status: z.enum(["pass", "warn", "fail"]),
      detail: z.string().optional(),
      issues: z.array(z.unknown()).optional(),
      info: z.array(z.string()).optional(),
    }),
  ),
});

export const StatusEventSchema = z.object({
  projectRoot: z.string().nullable(),
  hasManifest: z.boolean(),
  scope: z.string(),
  fromV2State: z.boolean(),
  skills: z.array(z.unknown()),
  plugins: z.array(z.unknown()),
  taps: z.array(z.unknown()),
  drift: z.unknown().optional(),
});

// Placeholder schemas for the other --json commands. Tighten in follow-up.
export const FindEventSchema = z.unknown();
export const VerifyEventSchema = z.unknown();
export const TryEventSchema = z.unknown();
export const MigrateEventSchema = z.unknown();
export const InfoEventSchema = z.unknown();
export const TapListEventSchema = z.unknown();
export const TapInfoEventSchema = z.unknown();
export const ConfigGetEventSchema = z.unknown();
export const ToggleEventSchema = z.unknown();
export const EnableEventSchema = z.unknown();
export const DisableEventSchema = z.unknown();

export type InstallEvent = z.infer<typeof InstallEventSchema>;
export type UpdateEvent = z.infer<typeof UpdateEventSchema>;
export type SyncEvent = z.infer<typeof SyncEventSchema>;
export type DoctorEvent = z.infer<typeof DoctorEventSchema>;
export type StatusEvent = z.infer<typeof StatusEventSchema>;
```

**Implementation Notes**:
- The five commands with full schemas are the highest-traffic JSON consumers.
- Placeholder `z.unknown()` schemas allow Phase 41's JSON pipeline to function while
  follow-up work tightens shapes per command.
- Event kinds use `:` namespacing so a single Zod union per command is clean.

**Acceptance criteria**:
- [ ] All 16 `--json` commands have a schema export (full or placeholder).
- [ ] Tests can `safeParse` JSON output line-by-line against the relevant schema.

---

### Unit 9 — Migrate CLI commands to use Output (the big one)

Touch every CLI command that currently calls a format.ts helper, clack spinner, or direct
`process.stdout.write` / `process.stderr.write`. Per-file pattern:

```typescript
// BEFORE (post-Phase-40)
export const command = defineCommand({
  meta: { name, description },
  args: { /* ..., json: { type: "boolean", default: false } */ },
  async run({ args }) {
    if (args.json) {
      jsonLine({ ...result });
      return;
    }
    successLine(`Done`);
  },
});

// AFTER (Phase 41)
import { createOutput } from "../output";

export const command = defineCommand({
  meta: { name, description },
  args: { /* ..., json: { type: "boolean", default: false } */ },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: args.quiet ?? false });
    out.json({ kind: "command:done", ...result });
    out.success(`Done`);
  },
});
```

Spinner replacement:

```typescript
// BEFORE
import { spinner } from "@clack/prompts";
const s = spinner();
s.start("Cloning");
// ... work
s.message("Scanning");
s.stop("Cloned");

// AFTER
const p = out.progress("Cloning");
// ... work
p.update("Scanning");
p.succeed("Cloned");
```

Stop/restart pattern:

```typescript
// BEFORE
s.stop();          // before prompt
const ans = await select({ ... });
s.start("Resuming");

// AFTER
p.pause();
const ans = await select({ ... });
p.resume();
```

**Files to migrate** (grouped by complexity):

**Tier 1 — Simple (1 batch)**: completions, skills/unlink, skills/link, skills/toggle,
skills/move, tap/init, tap/remove, config/edit, plugin/remove, plugin/info, plugin/toggle,
config/set, config/get.

**Tier 2 — Medium (1 batch)**: tap/add, tap/info, tap/list, status, doctor, migrate,
config/telemetry, skills/info, skills/adopt, verify, try, info (top-level), config (wizard).

**Tier 3 — Complex (1 batch each)**:
- install.ts (spinners + prompts + step logger + capture callbacks)
- update.ts (3 spinners, per-skill callbacks, semantic spinner)
- find.ts (search prompt + table + interactive picker)
- tap/install.ts (spinner stack, picker)
- sync.ts (per-item progress callbacks)
- toggle.ts, enable.ts, disable.ts (multi-component picker)
- create.ts (multi-phase prompts)
- config/security.ts (longest interactive wizard)

**Also migrate**:
- `index.ts` — startup messages (v1 detection, update notice, telemetry banner) → `out.warn` / `out.info`.
- `try.ts` — 18 raw `process.stdout.write` → `out.info` / `out.json`.
- `taps.ts` (in core, 1 stderr write) — surface via callback so CLI emits.

**Acceptance criteria**:
- [ ] No file under `packages/cli/src/commands/` imports `successLine`, `errorLine`, `infoLine`, `jsonLine`, or `securityBlock` directly.
- [ ] No file under `packages/cli/src/commands/` calls `spinner()` from `@clack/prompts` directly.
- [ ] No file under `packages/cli/src/` calls `process.stdout.write` or `process.stderr.write` directly except inside `cli/src/output/` and `cli/src/completions/dynamic.ts`.
- [ ] `bun test packages/cli/src/commands/` passes.

---

### Unit 10 — Test rewrites

Existing tests use `runSkilltap` to capture stdout/stderr from a subprocess. That stays the
primary integration test pattern. Phase 41 adds:

1. **Output unit tests** — per adapter, assert on writes:
   - `tty.test.ts`: TTY adapter writes ANSI-colored output, spinner lifecycle.
   - `plain.test.ts`: plain adapter strips colors, no spinner output.
   - `json.test.ts`: JSON adapter emits valid NDJSON, every line parses.
   - `pick.test.ts`: pickMode resolution table.
   - `factory.test.ts`: factory returns the right adapter per mode.
   - `capture.test.ts`: CaptureOutput records every event kind.

2. **Schema tests** — per JSON command:
   - Run `runSkilltap [cmd] --json`, parse each line, assert it `safeParse`s against the schema.

3. **Existing test fixes**:
   - Tests that grep for "✓ " continue to work (TTY mode preserves the glyph).
   - Tests that grep for "OK:" or "SKIP:" prefixes (legacy from agent-mode era — should
     have been removed in Phase 40; if any survive, update them).
   - `runSkilltap` runs in pipe mode → plain output → no ANSI codes. Tests asserting on
     exact text work; tests asserting on ANSI patterns need updating.

**Acceptance criteria**:
- [ ] New tests under `packages/cli/src/output/*.test.ts` cover each adapter.
- [ ] At least one test per JSON command parses every emitted line through its Zod schema.
- [ ] `bun test` passes with no regression in count.

---

## Pre-Mortem

**Riskiest assumption**: That the migration from format.ts helpers to `Output` methods is
behavior-preserving in TTY mode. Tests that grep for specific output text could fail if the
spinner-vs-text or color-vs-no-color decision changes.

**Mitigation**:
- TTY adapter wraps format.ts directly — same text output for `success`/`error`/`info`.
- Spinner replacement uses the same clack spinner under the hood; output identical.
- Plain mode is a deliberate behavioral change for non-TTY callers (no colors). Tests under
  `runSkilltap` (which pipes) currently see colored output (because format.ts always colors).
  Post-Phase-41 they see plain text. Most assertions use `.toContain(text)` — substring still
  matches. A handful may need updating; budget time for this in Unit 10.

**What would have to be true to fail in production**:
- A user has a script greppping for ANSI codes (vanishingly unlikely; --json is the right tool).
- A spinner started in non-TTY context corrupts output (mitigated: plain adapter emits plain text, no escape codes).
- A nested command invocation (sync → install) creates two `Output` instances that fight (mitigated: each is an instance, no shared state).

**Fallback**: Keep `format.ts` and clack spinner imports legal during migration. Tier 1 → 2 → 3 batched migration with `bun test` between batches. If a tier fails, revert that tier only.

**Where I'm least sure**:
- Per-command JSON event schemas. Initial Phase 41 covers 5 commands fully and 11 with `z.unknown()`. The full coverage is follow-up work; this is acceptable for the "schema-validated per command" exit criteria as long as the contract is set.
- Whether `out.json()` should be the only JSON path or whether `out.success()` should also emit a `success` event in JSON mode. Decision: `out.success()` is human-facing and is a no-op in JSON mode; commands explicitly call `out.json({ kind: "...:done", ... })` when they want a structured "done" event. This is the explicit-is-better choice.

## Implementation Order

1. **Unit 1** (interface) and **Unit 2** (pickMode) — pure types, no consumers yet. Land in one commit.
2. **Units 3, 4, 5, 6** — three adapters + factory. Land in one commit; covered by adapter unit tests.
3. **Unit 7** (CaptureOutput) — small; land before Unit 9 so command tests can use it.
4. **Unit 8** (schemas) — schemas + types. Land before Unit 9 so commands can import them.
5. **Unit 9** (CLI migration) — three batches, tier 1 → 2 → 3. `bun test` between each.
6. **Unit 10** (test rewrites) — after each tier, fix any tests that broke.

Sequential dependency chain: 1 → 2 → 3,4,5,6 → 7,8 → 9 (in tiers) → 10.

Parallel implementation agents:
- Agent A: Units 1, 2, 3, 4, 5, 6, 7 (the foundation — single commit).
- Agent B: Unit 8 (schemas — separate commit).
- Agent C: Unit 9 tier 1 + 2 (medium-difficulty migration).
- Agent D (only if tier 3 is too large): Unit 9 tier 3 commands one at a time.

## Testing

### Unit Tests: `packages/cli/src/output/*.test.ts`

```
describe("pickMode")
  - explicit json: true → "json"
  - json: false + isTTY: true → "tty"
  - json: false + isTTY: false → "plain"
  - no opts + process.stdout.isTTY === true → "tty"
  - no opts + process.stdout.isTTY undefined → "plain"

describe("createTtyOutput")
  - success("X") writes "✓ X\n" to stdout (with ANSI green check)
  - error("X", "h") writes "error: X\n  hint: h\n" to stderr (with ANSI red error)
  - quiet=true: success and info no-op; error still fires
  - progress lifecycle: start, update, succeed
  - progress pause/resume: stops spinner, restarts on resume
  - json() is a no-op

describe("createPlainOutput")
  - no ANSI codes in any output (regex check)
  - success("X") writes "X\n" to stdout (plain)
  - progress: emits "X...\n" then "X done\n" or no-op when quiet
  - pause/resume no-op

describe("createJsonOutput")
  - success/info/block produce no output
  - error writes valid JSON line to stdout
  - json(event) writes valid JSON line to stdout
  - progress emits progress:start/update/done/fail events
  - every emission is followed by exactly one newline

describe("createCaptureOutput")
  - records every event kind exactly once per call
  - events array preserves call order
```

### Integration Tests

For each Tier-1/2 command after migration:
```
- runSkilltap [cmd] (no --json): asserts on plain text output
- runSkilltap [cmd] --json: parses each line, validates against Zod schema, asserts shape
```

For Tier-3 commands (install, update, sync, find, etc.): existing tests should continue
passing; add at minimum one test that parses the `--json` output line-by-line.

## Verification Checklist

```bash
# 1. Build
bun run build

# 2. Output module tests
bun test packages/cli/src/output/

# 3. CLI command tests
bun test packages/cli/src/commands/

# 4. Full suite
bun test

# 5. Source-side grep — direct stdout/stderr writes only inside output/ and completions/
grep -rn "process\.stdout\.write\|process\.stderr\.write" packages/cli/src/ --include="*.ts" | \
  grep -v ".test.ts" | grep -v "/output/" | grep -v "/completions/"
# Expect: empty.

# 6. Source-side grep — format.ts helpers no longer used by commands
grep -rn "successLine\|errorLine\|infoLine\|jsonLine\|securityBlock" \
  packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"
# Expect: empty.

# 7. Spot check: --json output is valid NDJSON
bun run dev doctor --json | while read -r line; do echo "$line" | jq . > /dev/null; done
echo "All lines parsed successfully"

# 8. Spot check: piped stdout has no ANSI codes
bun run dev status | cat | head -5 | od -c | grep -i "\\\\033" || echo "No ANSI codes in plain mode"
```

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Test regressions from ANSI-vs-plain output difference in piped mode | Medium | Most tests use `.toContain` substring matches; ANSI difference doesn't affect them. Budget Unit 10 effort. |
| Migration of complex commands (install, update, sync) introduces subtle progress bugs | Medium | Full parity test suite. Tier-3 batch can be implemented one command at a time with `bun test` between. |
| Per-command JSON schemas are placeholder for many commands | Low | Acceptable for "schema-validated per command" exit criteria; placeholder lets the pipeline work, follow-up tightens. |
| `out.progress()` semantics don't match every spinner-using command's needs | Low | The `pause`/`resume` design covers the install/update stop-restart pattern. If a command needs more, add a method to `Progress` interface as needed. |
| Nested command invocations create competing Output instances | Low | Each instance is independent; no shared state. Sync calling install internally just constructs its own Output (which it already does for callbacks today). |
| `try.ts`'s 18 raw stdout writes turn out to need a special block API | Medium | If `out.info` is too granular, add `out.block(lines)` (already in design); use that. |
