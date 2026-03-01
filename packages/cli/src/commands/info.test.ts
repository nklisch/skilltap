import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { installSkill } from "@skilltap/core";
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
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

  test("--json outputs valid JSON with installed skill", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runInfo(
        ["standalone-skill", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(parsed.name).toBe("standalone-skill");
      expect(parsed.scope).toBe("global");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("info — tap-available skill", () => {
  async function createLocalTap() {
    const tapDir = await makeTmpDir();
    const tapJson = {
      name: "test-tap",
      description: "Test tap",
      skills: [
        {
          name: "tap-only-skill",
          description: "A skill only in the tap",
          repo: "https://github.com/example/tap-only-skill",
          tags: ["test"],
        },
      ],
    };
    await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
    await initRepo(tapDir);
    await commitAll(tapDir);
    return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
  }

  test("shows (available) status for tap skill not installed", async () => {
    const tap = await createLocalTap();
    try {
      // Register the tap
      const proc = Bun.spawn(
        ["bun", "run", "--bun", "src/index.ts", "tap", "add", "test-tap", tap.path],
        {
          cwd: CLI_DIR,
          stdout: "pipe",
          stderr: "pipe",
          env: { ...process.env, SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir },
        },
      );
      await proc.exited;

      const { exitCode, stdout } = await runInfo(["tap-only-skill"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("(available)");
      expect(stdout).toContain("tap-only-skill");
    } finally {
      await tap.cleanup();
    }
  });

  test("--json outputs skill with status: available", async () => {
    const tap = await createLocalTap();
    try {
      const proc = Bun.spawn(
        ["bun", "run", "--bun", "src/index.ts", "tap", "add", "test-tap", tap.path],
        {
          cwd: CLI_DIR,
          stdout: "pipe",
          stderr: "pipe",
          env: { ...process.env, SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir },
        },
      );
      await proc.exited;

      const { exitCode, stdout } = await runInfo(
        ["tap-only-skill", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(parsed.name).toBe("tap-only-skill");
      expect(parsed.status).toBe("available");
      expect(parsed.tap).toBe("test-tap");
    } finally {
      await tap.cleanup();
    }
  });
});
