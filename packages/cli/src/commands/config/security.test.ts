import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";

setDefaultTimeout(60_000);

import { cliCmd, createTestEnv, type TestEnv } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../../..`;

async function runSecurity(
  args: string[],
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    [...cliCmd(), "config", "security", ...args],
    {
      cwd: CLI_DIR,
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

async function runGet(key: string, configDir: string): Promise<string> {
  const proc = Bun.spawn(
    [...cliCmd(), "config", "get", key],
    {
      cwd: CLI_DIR,
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  await proc.exited;
  return (await new Response(proc.stdout).text()).trim();
}

async function runGetJson(key: string, configDir: string): Promise<unknown> {
  const proc = Bun.spawn(
    [...cliCmd(), "config", "get", key, "--json"],
    {
      cwd: CLI_DIR,
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  await proc.exited;
  return JSON.parse(await new Response(proc.stdout).text());
}

let env: TestEnv;
let configDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  configDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
});

describe("skilltap config security (non-interactive)", () => {
  test("--scan semantic updates security.scan", async () => {
    const result = await runSecurity(["--scan", "semantic"], configDir);
    expect(result.exitCode).toBe(0);
    expect(await runGet("security.scan", configDir)).toBe("semantic");
  });

  test("--on-warn fail updates security.on_warn", async () => {
    const result = await runSecurity(["--on-warn", "fail"], configDir);
    expect(result.exitCode).toBe(0);
    expect(await runGet("security.on_warn", configDir)).toBe("fail");
  });

  test("--scan and --on-warn together apply both updates", async () => {
    const result = await runSecurity(
      ["--scan", "none", "--on-warn", "prompt"],
      configDir,
    );
    expect(result.exitCode).toBe(0);
    expect(await runGet("security.scan", configDir)).toBe("none");
    expect(await runGet("security.on_warn", configDir)).toBe("prompt");
  });

  test("--trust-add appends a glob pattern to security.trust", async () => {
    const result = await runSecurity(
      ["--trust-add", "github.com/me/*"],
      configDir,
    );
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("github.com/me/*");
    expect(await runGetJson("security.trust", configDir)).toEqual([
      "github.com/me/*",
    ]);
  });

  test("--trust-add does not duplicate existing patterns", async () => {
    await runSecurity(["--trust-add", "foo"], configDir);
    await runSecurity(["--trust-add", "foo"], configDir);
    expect(await runGetJson("security.trust", configDir)).toEqual(["foo"]);
  });

  test("multiple --trust-add invocations append patterns", async () => {
    await runSecurity(["--trust-add", "alpha"], configDir);
    await runSecurity(["--trust-add", "beta"], configDir);
    expect(await runGetJson("security.trust", configDir)).toEqual([
      "alpha",
      "beta",
    ]);
  });

  test("--trust-remove removes the matching pattern", async () => {
    await runSecurity(["--trust-add", "alpha"], configDir);
    await runSecurity(["--trust-add", "beta"], configDir);

    const result = await runSecurity(["--trust-remove", "alpha"], configDir);
    expect(result.exitCode).toBe(0);
    expect(await runGetJson("security.trust", configDir)).toEqual(["beta"]);
  });

  test("--trust-remove on missing pattern exits 1", async () => {
    const result = await runSecurity(
      ["--trust-remove", "nonexistent"],
      configDir,
    );
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("nonexistent");
  });

  test("--trust-list prints empty marker when no patterns", async () => {
    const result = await runSecurity(["--trust-list"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("(no trust patterns)");
  });

  test("--trust-list prints each pattern on its own line", async () => {
    await runSecurity(["--trust-add", "alpha"], configDir);
    await runSecurity(["--trust-add", "beta"], configDir);

    const result = await runSecurity(["--trust-list"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("alpha");
    expect(result.stdout).toContain("beta");
  });

  test("invalid --scan value exits 1", async () => {
    const result = await runSecurity(["--scan", "turbo"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr.toLowerCase()).toContain("scan");
  });

  test("invalid --on-warn value exits 1", async () => {
    const result = await runSecurity(["--on-warn", "yolo"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr.toLowerCase()).toContain("on-warn");
  });

  test("no flags in non-TTY exits 1 (TTY required for interactive)", async () => {
    const result = await runSecurity([], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("requires a TTY");
  });
});
