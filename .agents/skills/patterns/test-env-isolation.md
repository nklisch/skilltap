# Pattern: Test Environment Isolation

`createTestEnv()` from `@skilltap/test-utils` creates isolated temp directories for `SKILLTAP_HOME` and `XDG_CONFIG_HOME`, sets the env vars in-process, and returns a typed `cleanup()` that restores originals and removes the dirs.

## Rationale

Tests that call core functions must not read from or write to the real user config or skill store. Manually creating temp dirs and restoring env vars in every `beforeEach`/`afterEach` is repetitive and error-prone. `createTestEnv()` centralizes this boilerplate and guarantees restoration even when tests fail.

## Examples

### Example 1: createTestEnv — definition
**File**: `packages/test-utils/src/env.ts:23`
```typescript
export async function createTestEnv(): Promise<TestEnv> {
  const homeDir = await mkdtemp(join(tmpdir(), "skilltap-test-"));
  const configDir = await mkdtemp(join(tmpdir(), "skilltap-cfg-"));

  const savedHome = process.env.SKILLTAP_HOME;
  const savedXdg = process.env.XDG_CONFIG_HOME;

  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;

  return {
    homeDir,
    configDir,
    cleanup: async () => {
      // restores originals (or deletes if they were unset)
      ...
      await rm(homeDir, { recursive: true, force: true });
      await rm(configDir, { recursive: true, force: true });
    },
  };
}
```

### Example 2: Standard beforeEach/afterEach setup
**File**: `packages/cli/src/commands/install.test.ts:24`
```typescript
import { createTestEnv, runSkilltap, type TestEnv } from "@skilltap/test-utils";

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

### Example 3: Using homeDir/configDir with runSkilltap
```typescript
test("installs a skill globally", async () => {
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

### Example 4: Core tests — env vars control globalBase() / getConfigDir()
**File**: `packages/core/src/config.test.ts` (representative pattern)
```typescript
let env: TestEnv;
beforeEach(async () => { env = await createTestEnv(); });
afterEach(async () => { await env.cleanup(); });

test("loadConfig returns default when no config file exists", async () => {
  const result = await loadConfig();
  expect(result.ok).toBe(true);
  // env.configDir is the XDG_CONFIG_HOME — loadConfig reads from there
});
```

## When to Use

- Any test that calls core functions touching `state.json`, `config.toml`, or any path under `SKILLTAP_HOME`/`XDG_CONFIG_HOME`
- CLI subprocess tests — pass `env.homeDir` and `env.configDir` to `runSkilltap()`
- Whenever you'd manually set and restore `SKILLTAP_HOME`/`XDG_CONFIG_HOME`

## When NOT to Use

- Pure function unit tests with no I/O — no env isolation needed
- Tests that only need an extra temp dir (not the global state dirs) — use `makeTmpDir()` from test-utils for that extra dir
- Schema validation tests — use inline `VALID_*` constants, no dirs needed

## Common Violations

- Manual `beforeEach`/`afterEach` with `mkdtemp` and env var save/restore — replace with `createTestEnv()`
- Not calling `env.cleanup()` in `afterEach` — temp dirs accumulate and env vars bleed into subsequent tests
- Accessing `env.homeDir` or `env.configDir` before `beforeEach` runs — declare `let env: TestEnv` and assign in `beforeEach`
- Using `createTestEnv()` for CLI subprocess tests without passing the dirs to `runSkilltap()` — subprocess gets its own env without `SKILLTAP_HOME`
