import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(15_000);
import { join } from "node:path";
import {
  commitAll,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

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
  test("shows no taps message when empty", async () => {
    const { exitCode, stdout } = await runCli(
      ["tap", "list"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No taps configured");
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

      // Verify it's gone from list
      const { stdout: listOut } = await runCli(
        ["tap", "list"],
        homeDir,
        configDir,
      );
      expect(listOut).toContain("No taps configured");
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
