import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runConfig(
  args: string[],
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "config", ...args],
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
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  delete process.env.XDG_CONFIG_HOME;
  await removeTmpDir(configDir);
});

describe("skilltap config", () => {
  test("rejects non-TTY", async () => {
    const { exitCode, stderr } = await runConfig([], configDir);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("must be run interactively");
  });
});
