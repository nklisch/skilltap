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

async function runGet(
  args: string[],
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn([...cliCmd(), "config", "get", ...args], {
    cwd: CLI_DIR,
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      XDG_CONFIG_HOME: configDir,
    },
  });
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
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

describe("skilltap config get", () => {
  test("gets a string field (defaults.scope)", async () => {
    const { exitCode, stdout } = await runGet(["defaults.scope"], configDir);
    expect(exitCode).toBe(0);
    // Default scope is "" so output is just a newline
    expect(stdout).toBe("\n");
  });

  test("gets a boolean field (defaults.yes)", async () => {
    const { exitCode, stdout } = await runGet(["defaults.yes"], configDir);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("false");
  });

  test("gets a number field (updates.interval_hours)", async () => {
    const { exitCode, stdout } = await runGet(
      ["updates.interval_hours"],
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("24");
  });

  test("gets security.scan default", async () => {
    const { exitCode, stdout } = await runGet(["security.scan"], configDir);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("static");
  });

  test("errors on unknown key", async () => {
    const { exitCode, stderr } = await runGet(["nonexistent.key"], configDir);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("Unknown config key");
  });

  test("--json returns full config as valid JSON", async () => {
    const { exitCode, stdout } = await runGet(["--json"], configDir);
    expect(exitCode).toBe(0);
    const config = JSON.parse(stdout);
    expect(config.defaults).toBeDefined();
    expect(config.security).toBeDefined();
  });

  test("--json with key returns single value as JSON", async () => {
    const { exitCode, stdout } = await runGet(
      ["defaults.yes", "--json"],
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(JSON.parse(stdout)).toBe(false);
  });

  test("no key without --json prints flat key=value lines", async () => {
    const { exitCode, stdout } = await runGet([], configDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("defaults.scope =");
    expect(stdout).toContain("scanner.threshold = 5");
  });

  test("gets security.on_warn default", async () => {
    const { exitCode, stdout } = await runGet(["security.on_warn"], configDir);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("install");
  });

  test("gets security.trust default (empty array)", async () => {
    const { exitCode, stdout } = await runGet(
      ["security.trust", "--json"],
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(JSON.parse(stdout)).toEqual([]);
  });

  test("gets scanner.agent_cli default", async () => {
    const { exitCode, stdout } = await runGet(["scanner.agent_cli"], configDir);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });

  test("gets security section as object", async () => {
    const { exitCode, stdout } = await runGet(
      ["security", "--json"],
      configDir,
    );
    expect(exitCode).toBe(0);
    const sec = JSON.parse(stdout);
    expect(sec.scan).toBeDefined();
    expect(sec.on_warn).toBeDefined();
    expect(sec.scan).toBe("static");
    expect(sec.on_warn).toBe("install");
    expect(sec.trust).toEqual([]);
  });
});
