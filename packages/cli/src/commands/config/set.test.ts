import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(60_000);
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../../..`;

async function runCmd(
  subCmd: string,
  args: string[],
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "config", subCmd, ...args],
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

let configDir: string;

beforeEach(async () => {
  configDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(configDir);
});

describe("skilltap config set", () => {
  test("sets a string enum and reads it back", async () => {
    const set = await runCmd("set", ["defaults.scope", "global"], configDir);
    expect(set.exitCode).toBe(0);
    expect(set.stdout).toBe("OK: defaults.scope = global\n");

    const get = await runCmd("get", ["defaults.scope"], configDir);
    expect(get.exitCode).toBe(0);
    expect(get.stdout.trim()).toBe("global");
  });

  test("sets a boolean field", async () => {
    const set = await runCmd("set", ["defaults.yes", "true"], configDir);
    expect(set.exitCode).toBe(0);

    const get = await runCmd("get", ["defaults.yes", "--json"], configDir);
    expect(JSON.parse(get.stdout)).toBe(true);
  });

  test("sets a number field", async () => {
    const set = await runCmd(
      "set",
      ["updates.interval_hours", "48"],
      configDir,
    );
    expect(set.exitCode).toBe(0);

    const get = await runCmd("get", ["updates.interval_hours"], configDir);
    expect(get.stdout.trim()).toBe("48");
  });

  test("sets an array field with multiple values", async () => {
    const set = await runCmd(
      "set",
      ["defaults.also", "claude-code", "cursor"],
      configDir,
    );
    expect(set.exitCode).toBe(0);

    const get = await runCmd("get", ["defaults.also"], configDir);
    expect(get.stdout.trim()).toBe("claude-code cursor");
  });

  test("sets an array field with single value", async () => {
    const set = await runCmd(
      "set",
      ["defaults.also", "claude-code"],
      configDir,
    );
    expect(set.exitCode).toBe(0);

    const get = await runCmd("get", ["defaults.also", "--json"], configDir);
    expect(JSON.parse(get.stdout)).toEqual(["claude-code"]);
  });

  test("clears an array field with no values", async () => {
    // First set some values
    await runCmd("set", ["defaults.also", "claude-code"], configDir);

    // Then clear
    const set = await runCmd("set", ["defaults.also"], configDir);
    expect(set.exitCode).toBe(0);

    const get = await runCmd("get", ["defaults.also", "--json"], configDir);
    expect(JSON.parse(get.stdout)).toEqual([]);
  });

  test("validates enum values", async () => {
    const set = await runCmd(
      "set",
      ["updates.auto_update", "major"],
      configDir,
    );
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("Invalid value");
    expect(set.stderr).toContain("off");
  });

  test("rejects blocked key (agent-mode.enabled)", async () => {
    const set = await runCmd(
      "set",
      ["agent-mode.enabled", "true"],
      configDir,
    );
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("cannot be set");
    expect(set.stderr).toContain("config agent-mode");
  });

  test("rejects blocked key (security.scan)", async () => {
    const set = await runCmd("set", ["security.scan", "off"], configDir);
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("cannot be set");
  });

  test("sets default_git_host and reads it back", async () => {
    const set = await runCmd(
      "set",
      ["default_git_host", "https://gitea.example.com"],
      configDir,
    );
    expect(set.exitCode).toBe(0);
    expect(set.stdout).toContain("OK: default_git_host = https://gitea.example.com");

    const get = await runCmd("get", ["default_git_host"], configDir);
    expect(get.exitCode).toBe(0);
    expect(get.stdout.trim()).toBe("https://gitea.example.com");
  });

  test("default_git_host defaults to https://github.com", async () => {
    const get = await runCmd("get", ["default_git_host"], configDir);
    expect(get.exitCode).toBe(0);
    expect(get.stdout.trim()).toBe("https://github.com");
  });

  test("rejects unknown key", async () => {
    const set = await runCmd("set", ["foo.bar", "baz"], configDir);
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("Unknown or non-settable");
    expect(set.stderr).toContain("Settable keys");
  });

  test("rejects invalid boolean value", async () => {
    const set = await runCmd("set", ["defaults.yes", "maybe"], configDir);
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("Invalid boolean");
  });

  test("rejects non-integer number", async () => {
    const set = await runCmd(
      "set",
      ["updates.interval_hours", "3.5"],
      configDir,
    );
    expect(set.exitCode).toBe(1);
    expect(set.stderr).toContain("Invalid integer");
  });

  test("preserves other config values when setting", async () => {
    // Set scope
    await runCmd("set", ["defaults.scope", "global"], configDir);
    // Set also (different section field)
    await runCmd("set", ["defaults.also", "cursor"], configDir);

    // Both should be preserved
    const get1 = await runCmd("get", ["defaults.scope"], configDir);
    expect(get1.stdout.trim()).toBe("global");

    const get2 = await runCmd("get", ["defaults.also"], configDir);
    expect(get2.stdout.trim()).toBe("cursor");
  });
});
