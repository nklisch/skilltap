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
  test("--preset strict applies security settings", async () => {
    const result = await runSecurity(["--preset", "strict"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("strict");

    expect(await runGet("security.scan", configDir)).toBe("semantic");
    expect(await runGet("security.on_warn", configDir)).toBe("fail");
    expect(await runGet("security.require_scan", configDir)).toBe("true");
  });

  test("--preset relaxed applies relaxed settings", async () => {
    const result = await runSecurity(["--preset", "relaxed"], configDir);
    expect(result.exitCode).toBe(0);

    expect(await runGet("security.scan", configDir)).toBe("static");
    expect(await runGet("security.on_warn", configDir)).toBe("allow");
  });

  test("--trust tap:foo=none adds override to config", async () => {
    const result = await runSecurity(["--trust", "tap:foo=none"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("added tap trust override 'foo' → none");
  });

  test("--trust source:npm=standard adds source override", async () => {
    const result = await runSecurity(
      ["--trust", "source:npm=standard"],
      configDir,
    );
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain(
      "added source trust override 'npm' → standard",
    );
  });

  test("--remove-trust removes matching override", async () => {
    // First add an override
    await runSecurity(["--trust", "tap:foo=none"], configDir);

    const result = await runSecurity(["--remove-trust", "foo"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("removed trust override 'foo'");
  });

  test("invalid preset name exits 1", async () => {
    const result = await runSecurity(["--preset", "bogus"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid preset");
    expect(result.stderr).toContain("bogus");
  });

  test("invalid trust format exits 1", async () => {
    const result = await runSecurity(["--trust", "badformat"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid --trust format");
  });

  test("no flags in non-TTY exits 1 (TTY required for interactive)", async () => {
    const result = await runSecurity([], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("requires a TTY");
  });

  test("--scan and --on-warn apply individual field overrides", async () => {
    const result = await runSecurity(
      ["--scan", "off", "--on-warn", "allow"],
      configDir,
    );
    expect(result.exitCode).toBe(0);

    expect(await runGet("security.scan", configDir)).toBe("off");
    expect(await runGet("security.on_warn", configDir)).toBe("allow");
  });

  test("--remove-trust nonexistent exits 1", async () => {
    const result = await runSecurity(
      ["--remove-trust", "nonexistent"],
      configDir,
    );
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("No trust override found");
  });

  test("--require-scan flag sets require_scan", async () => {
    const result = await runSecurity(["--require-scan"], configDir);
    expect(result.exitCode).toBe(0);
    expect(await runGet("security.require_scan", configDir)).toBe("true");
  });

  test("invalid --scan value exits 1", async () => {
    const result = await runSecurity(["--scan", "turbo"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid scan");
  });

  test("invalid --on-warn value exits 1", async () => {
    const result = await runSecurity(["--on-warn", "yolo"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid on-warn");
  });

  test("--trust with invalid preset in trust string exits 1", async () => {
    const result = await runSecurity(["--trust", "tap:foo=bogus"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid --trust format");
  });

  test("--trust with invalid source type exits 1", async () => {
    const result = await runSecurity(
      ["--trust", "source:invalid=none"],
      configDir,
    );
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid --trust format");
  });

  test("--preset with --scan applies preset then overrides scan", async () => {
    const result = await runSecurity(
      ["--preset", "relaxed", "--scan", "semantic"],
      configDir,
    );
    expect(result.exitCode).toBe(0);
    // scan should be semantic (flag overrides preset), on_warn should be allow (from relaxed)
    expect(await runGet("security.scan", configDir)).toBe("semantic");
    expect(await runGet("security.on_warn", configDir)).toBe("allow");
  });

  test("multiple trust overrides can be added sequentially", async () => {
    await runSecurity(["--trust", "tap:corp=none"], configDir);
    await runSecurity(["--trust", "source:npm=strict"], configDir);

    const proc = Bun.spawn(
      [
        "bun",
        "run",
        "--bun",
        "src/index.ts",
        "config",
        "get",
        "security",
        "--json",
      ],
      {
        cwd: CLI_DIR,
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
        env: { ...process.env, XDG_CONFIG_HOME: configDir },
      },
    );
    await proc.exited;
    const secJson = JSON.parse(await new Response(proc.stdout).text());
    expect(secJson.overrides).toHaveLength(2);
    expect(secJson.overrides[0].match).toBe("corp");
    expect(secJson.overrides[1].match).toBe("npm");
  });
});
