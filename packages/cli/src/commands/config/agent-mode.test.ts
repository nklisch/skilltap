import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(45_000);
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../../..`;

async function runAgentMode(
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "config", "agent-mode"],
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

describe("skilltap config agent-mode", () => {
  test("rejects non-TTY with correct message", async () => {
    const { exitCode, stderr } = await runAgentMode(configDir);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("must be run interactively");
    expect(stderr).toContain("only be enabled or disabled by a human");
  });
});
