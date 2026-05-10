# Pattern: CLI Subprocess Testing

CLI integration tests use `runSkilltap()` and `createTestEnv()` from `@skilltap/test-utils`. These helpers isolate tests from the real user config and route through the compiled binary when `SKILLTAP_TEST_BIN` is set.

## Rationale

CLI commands use `@clack/prompts`, `process.exit()`, and write directly to stdout/stderr — they can't be imported and called as functions in tests. Spawning a subprocess ensures the full CLI pipeline runs. `runSkilltap()` wraps `cliCmd()` which routes to either the compiled binary (when `SKILLTAP_TEST_BIN` is set for binary verification) or `bun run --bun src/index.ts` (default, source mode). `createTestEnv()` provides isolated temp dirs and sets the env vars automatically.

## Examples

### Example 1: Test file boilerplate — imports and env setup
**File**: `packages/cli/src/commands/install.test.ts:1`
```typescript
import {
  createTestEnv,
  createStandaloneSkillRepo,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

let env: TestEnv;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
});
```

### Example 2: Running a command and asserting on exit code + output
**File**: `packages/cli/src/commands/install.test.ts:39`
```typescript
test("installs with --yes --scope global and shows success", async () => {
  const repo = await createStandaloneSkillRepo();
  try {
    const { exitCode, stdout } = await runSkilltap(
      ["install", "skill", repo.path, "--yes", "--scope", "global", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
  } finally {
    await repo.cleanup();
  }
});
```

### Example 3: runSkilltap and cliCmd — how they work
**File**: `packages/test-utils/src/cli.ts:11`
```typescript
export function cliCmd(): string[] {
  const bin = process.env.SKILLTAP_TEST_BIN;
  if (bin && bin.length > 0) return [bin];          // compiled binary path
  return ["bun", "run", "--bun", CLI_ENTRY];         // source mode
}

export async function runSkilltap(
  args: string[],
  homeDir: string,
  configDir: string,
  cwd: string = homeDir,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn([...cliCmd(), ...args], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      SKILLTAP_HOME: homeDir,
      XDG_CONFIG_HOME: configDir,
      SKILLTAP_NO_STARTUP: "1",
    },
  });
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}
```

### Example 4: Non-TTY detection via stdin: "pipe" (direct Bun.spawn)
When testing interactive-only commands that must reject non-TTY contexts, use `cliCmd()` with `Bun.spawn` and `stdin: "pipe"`:
```typescript
import { cliCmd } from "@skilltap/test-utils";

const proc = Bun.spawn(
  [...cliCmd(), "config", "agent-mode"],
  {
    stdin: "pipe",   // forces non-TTY
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, XDG_CONFIG_HOME: configDir },
  },
);
expect(await proc.exited).toBe(1);
expect(await new Response(proc.stderr).text()).toContain("must be run interactively");
```

### Example 5: JSON output tests
```typescript
test("--json flag outputs structured event", async () => {
  const { exitCode, stdout } = await runSkilltap(
    ["install", "skill", repo.path, "--yes", "--scope", "global", "--json"],
    homeDir,
    configDir,
  );
  expect(exitCode).toBe(0);
  const events = stdout.trim().split("\n").map((l) => JSON.parse(l));
  expect(events.some((e) => e.type === "install")).toBe(true);
});
```

## When to Use

- Testing any CLI command end-to-end (argument parsing through output)
- Verifying exit codes, stdout/stderr text, and filesystem side-effects
- For clack spinners, prompt interactions, TUI — use `runInteractive` (PTY) from test-utils instead

## When NOT to Use

- Unit testing core functions — import and call directly, no subprocess needed
- Testing individual UI helpers — import from `ui/` modules directly
- Don't hardcode `["bun", "run", "--bun", "src/index.ts"]` — always use `cliCmd()` so binary verification works

## Common Violations

- Hardcoding `["bun", "run", "--bun", "src/index.ts", ...]` instead of `cliCmd()` — test won't participate in `bun run verify:binary:tests`
- Manual `beforeEach/afterEach` with `mkdtemp` + env var save/restore — use `createTestEnv()` instead
- Forgetting `SKILLTAP_NO_STARTUP: "1"` when using raw `Bun.spawn` — startup telemetry check adds latency
- Not cleaning up fixture repos in `finally` — leaked temp dirs accumulate under `/tmp`
- Using `runSkilltap` for clack-rendered output — it runs in pipe mode, `isTTY` is false; use `runInteractive` for spinner/prompt output
