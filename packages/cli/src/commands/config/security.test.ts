import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(45_000);
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../../..`;

async function runSecurity(
  args: string[],
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "config", "security", ...args],
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

async function runGet(
  key: string,
  configDir: string,
): Promise<string> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "config", "get", key],
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

let configDir: string;

beforeEach(async () => {
  configDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(configDir);
});

describe("skilltap config security (non-interactive)", () => {
  test("--preset strict applies to both modes", async () => {
    const result = await runSecurity(["--preset", "strict"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("security.human = strict");
    expect(result.stdout).toContain("security.agent = strict");

    expect(await runGet("security.human.scan", configDir)).toBe("semantic");
    expect(await runGet("security.human.on_warn", configDir)).toBe("fail");
    expect(await runGet("security.human.require_scan", configDir)).toBe("true");
    expect(await runGet("security.agent.scan", configDir)).toBe("semantic");
    expect(await runGet("security.agent.on_warn", configDir)).toBe("fail");
    expect(await runGet("security.agent.require_scan", configDir)).toBe("true");
  });

  test("--preset strict --mode agent only changes agent mode", async () => {
    // First set human to relaxed
    await runSecurity(["--preset", "relaxed", "--mode", "human"], configDir);

    const result = await runSecurity(["--preset", "strict", "--mode", "agent"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("security.agent = strict");
    expect(result.stdout).not.toContain("security.human");

    // Human should still be relaxed
    expect(await runGet("security.human.scan", configDir)).toBe("static");
    expect(await runGet("security.human.on_warn", configDir)).toBe("allow");
    // Agent should be strict
    expect(await runGet("security.agent.scan", configDir)).toBe("semantic");
    expect(await runGet("security.agent.on_warn", configDir)).toBe("fail");
    expect(await runGet("security.agent.require_scan", configDir)).toBe("true");
  });

  test("--trust tap:foo=none adds override to config", async () => {
    const result = await runSecurity(["--trust", "tap:foo=none"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("added tap trust override 'foo' → none");
  });

  test("--trust source:npm=standard adds source override", async () => {
    const result = await runSecurity(["--trust", "source:npm=standard"], configDir);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("added source trust override 'npm' → standard");
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

  test("invalid mode exits 1", async () => {
    const result = await runSecurity(["--preset", "strict", "--mode", "invalid"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("Invalid mode");
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
      ["--mode", "human", "--scan", "off", "--on-warn", "allow"],
      configDir,
    );
    expect(result.exitCode).toBe(0);

    expect(await runGet("security.human.scan", configDir)).toBe("off");
    expect(await runGet("security.human.on_warn", configDir)).toBe("allow");
  });

  test("--remove-trust nonexistent exits 1", async () => {
    const result = await runSecurity(["--remove-trust", "nonexistent"], configDir);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("No trust override found");
  });
});
