import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { installSkill } from "@skilltap/core";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runInfo(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "info", ...args],
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

describe("info — not found", () => {
  test("exits 1 with error message", async () => {
    const { exitCode, stderr } = await runInfo(
      ["nonexistent-skill"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});

describe("info — installed skill", () => {
  test("shows skill details", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runInfo(
        ["standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
      expect(stdout).toContain("global");
    } finally {
      await repo.cleanup();
    }
  });

  test("shows sha (truncated to 7 chars)", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runInfo(
        ["standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("sha:");
    } finally {
      await repo.cleanup();
    }
  });
});
