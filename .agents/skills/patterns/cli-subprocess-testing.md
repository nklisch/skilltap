# Pattern: CLI Subprocess Testing

CLI integration tests spawn the CLI as a real subprocess using `Bun.spawn`, passing `SKILLTAP_HOME` and `XDG_CONFIG_HOME` env vars for isolation.

## Rationale

CLI commands use `@clack/prompts`, `process.exit()`, and write directly to stdout/stderr — they can't be imported and called as functions in tests. Spawning a subprocess ensures the full CLI pipeline runs (argument parsing, config loading, git operations, output formatting) while env var overrides keep tests isolated from the user's real config and installed skills.

## Examples

### Example 1: runCommand helper pattern
**File**: `packages/cli/src/commands/install.test.ts:15`
```typescript
const CLI_DIR = `${import.meta.dir}/../..`;

async function runInstall(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "install", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}
```

### Example 2: Test setup with temp dirs
**File**: `packages/cli/src/commands/install.test.ts:37`
```typescript
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  delete process.env.SKILLTAP_HOME;
  delete process.env.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});
```

### Example 3: Testing exit codes and output
**File**: `packages/cli/src/commands/install.test.ts:55`
```typescript
test("installs with --yes --global and shows success", async () => {
  const repo = await createStandaloneSkillRepo();
  try {
    const { exitCode, stdout } = await runInstall(
      [repo.path, "--yes", "--global", "--skip-scan"],
      homeDir, configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
  } finally {
    await repo.cleanup();
  }
});
```

### Example 4: Non-TTY detection via stdin: "pipe"
**File**: `packages/cli/src/commands/config/agent-mode.test.ts:8`
```typescript
const proc = Bun.spawn(
  ["bun", "run", "--bun", "src/index.ts", "config", "agent-mode"],
  {
    cwd: CLI_DIR,
    stdin: "pipe",      // ← forces non-TTY
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, XDG_CONFIG_HOME: configDir },
  },
);
// ...
expect(exitCode).toBe(1);
expect(stderr).toContain("must be run interactively");
```

## When to Use

- Testing any CLI command end-to-end (argument parsing through output)
- Testing agent mode behavior (write config with `agent-mode.enabled = true` before spawning)
- Testing non-TTY detection (use `stdin: "pipe"` to simulate piped input)

## When NOT to Use

- Unit testing core functions — import and call directly, no subprocess needed
- Testing individual UI helpers — import from `ui/` modules directly
- Don't use for tests that only need config/schema validation

## Common Violations

- Using `__dirname` instead of `import.meta.dir` — Bun uses `import.meta.dir`
- Wrong `CLI_DIR` depth — nested test files (e.g., `config/agent-mode.test.ts`) need `../../../` not `../../`
- Forgetting `--bun` flag in `bun run --bun src/index.ts` — ensures Bun runtime, not Node
- Not passing env vars to subprocess — tests will use real config without `SKILLTAP_HOME`/`XDG_CONFIG_HOME`
- Not cleaning up fixture repos in `finally` blocks — leaked temp dirs accumulate
