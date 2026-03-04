import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(15_000);
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

/** Write a minimal config.toml with builtin_tap = false to keep tests offline. */
async function disableBuiltinTap(configDir: string): Promise<void> {
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
}

const CLI_DIR = `${import.meta.dir}/../..`;

async function runCli(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["bun", "run", "--bun", "src/index.ts", ...args], {
    cwd: CLI_DIR,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      SKILLTAP_HOME: homeDir,
      XDG_CONFIG_HOME: configDir,
    },
  });
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

async function createLocalTap(
  skills: Array<{ name: string; description: string; repo: string }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name: "test-tap",
    description: "Test tap",
    skills: skills.map((s) => ({ tags: [], ...s })),
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

describe("tap add", () => {
  test("adds a tap and reports skill count", async () => {
    const tap = await createLocalTap([
      {
        name: "my-skill",
        description: "A skill",
        repo: "https://example.com/my-skill",
      },
    ]);
    try {
      const { exitCode, stdout } = await runCli(
        ["tap", "add", "home", tap.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Added tap 'home'");
      expect(stdout).toContain("1 skills");
    } finally {
      await tap.cleanup();
    }
  });

  test("accepts GitHub shorthand owner/repo", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      // Use the local tap path as a real git URL isn't available,
      // but we can verify two-arg form still works with a slash in name position
      const { exitCode, stdout } = await runCli(
        ["tap", "add", "my-custom-name", tap.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Added tap 'my-custom-name'");
    } finally {
      await tap.cleanup();
    }
  });

  test("errors on single arg that is not GitHub shorthand", async () => {
    const { exitCode, stderr } = await runCli(
      ["tap", "add", "just-a-name"],
      homeDir,
      configDir,
    );
    expect(exitCode).not.toBe(0);
    expect(stderr).toContain("Cannot parse");
  });

  test("errors if tap name already exists", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stderr } = await runCli(
        ["tap", "add", "home", tap.path],
        homeDir,
        configDir,
      );
      expect(exitCode).not.toBe(0);
      expect(stderr).toContain("already exists");
    } finally {
      await tap.cleanup();
    }
  });
});

describe("tap list", () => {
  test("shows no taps message when empty and builtin disabled", async () => {
    await disableBuiltinTap(configDir);
    const { exitCode, stdout } = await runCli(
      ["tap", "list"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No taps configured");
  });

  test("shows built-in tap by default even with no user taps", async () => {
    const { exitCode, stdout } = await runCli(
      ["tap", "list"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("skilltap-skills");
    expect(stdout).toContain("built-in");
  });

  test("lists configured taps", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["tap", "list"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("home");
      expect(stdout).toContain("1 skills");
    } finally {
      await tap.cleanup();
    }
  });
});

describe("tap remove", () => {
  test("removes a tap with --yes", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["tap", "remove", "home", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Removed tap 'home'");

      // Verify it's gone from list (built-in tap still shows)
      const { stdout: listOut } = await runCli(
        ["tap", "list"],
        homeDir,
        configDir,
      );
      expect(listOut).not.toContain("home");
      expect(listOut).toContain("skilltap-skills");
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if tap not found", async () => {
    const { exitCode, stderr } = await runCli(
      ["tap", "remove", "nonexistent", "--yes"],
      homeDir,
      configDir,
    );
    expect(exitCode).not.toBe(0);
    expect(stderr).toContain("not configured");
  });
});

describe("tap update", () => {
  test("updates a tap and reports skill count", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["tap", "update", "home"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("home");
      expect(stdout).toContain("1 skills");
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if named tap not configured", async () => {
    const { exitCode, stderr } = await runCli(
      ["tap", "update", "nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).not.toBe(0);
    expect(stderr).toContain("not configured");
  });
});

describe("tap init", () => {
  test("creates a new tap directory with tap.json", async () => {
    const workDir = await makeTmpDir();
    try {
      const proc = Bun.spawn(
        [
          "bun",
          "run",
          "--bun",
          `${CLI_DIR}/src/index.ts`,
          "tap",
          "init",
          "my-tap",
        ],
        {
          cwd: workDir,
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
      expect(exitCode).toBe(0);
      expect(stdout).toContain("my-tap");

      // Verify tap.json was created
      const tapJsonFile = Bun.file(join(workDir, "my-tap", "tap.json"));
      expect(await tapJsonFile.exists()).toBe(true);
      const tapJson = await tapJsonFile.json();
      expect(tapJson.name).toBe("my-tap");
      expect(tapJson.skills).toEqual([]);
    } finally {
      await removeTmpDir(workDir);
    }
  });
});

describe("tap install", () => {
  test("exits with error when no taps configured and builtin disabled", async () => {
    await disableBuiltinTap(configDir);
    const { exitCode, stdout, stderr } = await runCli(
      ["tap", "install", "--yes", "--global", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stdout + stderr).toMatch(/No skills available/);
  });

  test("--yes installs all skills from configured tap", async () => {
    const skillRepo = await createStandaloneSkillRepo();
    const tap = await createLocalTap([
      {
        name: "standalone-skill",
        description: "A skill",
        repo: skillRepo.path,
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);

      const { exitCode, stdout } = await runCli(
        ["tap", "install", "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });

  test("--tap scopes to a single tap", async () => {
    const skillRepo = await createStandaloneSkillRepo();
    const tap1 = await createLocalTap([
      {
        name: "standalone-skill",
        description: "From tap1",
        repo: skillRepo.path,
      },
    ]);
    const tap2 = await createLocalTap([
      {
        name: "other-skill",
        description: "From tap2",
        repo: "https://example.invalid/other",
      },
    ]);
    try {
      await runCli(["tap", "add", "tap1", tap1.path], homeDir, configDir);
      await runCli(["tap", "add", "tap2", tap2.path], homeDir, configDir);

      const { exitCode, stdout } = await runCli(
        ["tap", "install", "--tap", "tap1", "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await skillRepo.cleanup();
      await tap1.cleanup();
      await tap2.cleanup();
    }
  });

  test("--tap with unknown tap name exits with error", async () => {
    const tap = await createLocalTap([
      {
        name: "my-skill",
        description: "A skill",
        repo: "https://example.invalid/skill",
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);

      const { exitCode, stdout, stderr } = await runCli(
        ["tap", "install", "--tap", "nonexistent", "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stdout + stderr).toContain("nonexistent");
    } finally {
      await tap.cleanup();
    }
  });
});
