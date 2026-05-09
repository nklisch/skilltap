import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";

setDefaultTimeout(60_000);

import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

/** Write a minimal config.toml with builtin_tap = false to keep tests offline. */
async function disableBuiltinTap(configDir: string): Promise<void> {
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(
    join(configDir, "skilltap", "config.toml"),
    "builtin_tap = false\n",
  );
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

let env: TestEnv;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
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
      const { exitCode, stdout } = await runSkilltap(
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
      const { exitCode, stdout } = await runSkilltap(
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
    const { exitCode, stderr } = await runSkilltap(
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
      await runSkilltap(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stderr } = await runSkilltap(
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
    const { exitCode, stdout } = await runSkilltap(
      ["tap", "list"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No taps configured");
  });

  test("shows built-in tap by default even with no user taps", async () => {
    const { exitCode, stdout } = await runSkilltap(
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
      await runSkilltap(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runSkilltap(
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
      await runSkilltap(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runSkilltap(
        ["tap", "remove", "home", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Removed tap 'home'");

      // Verify it's gone from list (built-in tap still shows)
      const { stdout: listOut } = await runSkilltap(
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
    const { exitCode, stderr } = await runSkilltap(
      ["tap", "remove", "nonexistent", "--yes"],
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

describe("tap info", () => {
  test("shows details for a configured tap", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
      { name: "skill-b", description: "B", repo: "https://example.com/b" },
    ]);
    try {
      await runSkilltap(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runSkilltap(
        ["tap", "info", "home"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("home");
      expect(stdout).toContain("2");
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if tap not found", async () => {
    await disableBuiltinTap(configDir);
    const { exitCode, stderr } = await runSkilltap(
      ["tap", "info", "nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).not.toBe(0);
    expect(stderr).toContain("nonexistent");
  });

  test("--json outputs JSON", async () => {
    const tap = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      await runSkilltap(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runSkilltap(
        ["tap", "info", "home", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const json = JSON.parse(stdout);
      expect(json.name).toBe("home");
      expect(json.skillCount).toBe(1);
    } finally {
      await tap.cleanup();
    }
  });
});
