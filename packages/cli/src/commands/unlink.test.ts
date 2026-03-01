import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { linkSkill, loadInstalled } from "@skilltap/core";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runUnlink(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "unlink", ...args],
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

describe("unlink — not found", () => {
  test("exits 1 when skill not linked", async () => {
    const { exitCode, stderr } = await runUnlink(
      ["nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not linked");
  });
});

describe("unlink — linked skill", () => {
  test("removes the symlink", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await linkSkill(repo.path, { scope: "global" });

      const symlinkPath = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(symlinkPath)
          .then(() => true)
          .catch(() => false),
      ).toBe(true);

      const { exitCode, stdout } = await runUnlink(
        ["standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Unlinked");

      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("removes skill from installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await linkSkill(repo.path, { scope: "global" });
      await runUnlink(["standalone-skill"], homeDir, configDir);

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});
