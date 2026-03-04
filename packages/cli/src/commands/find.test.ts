import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";

setDefaultTimeout(15_000);
import { mkdir } from "node:fs/promises";
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

async function writeConfig(configDir: string, toml: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), toml);
}

async function createLocalTap(
  skills: Array<{
    name: string;
    description: string;
    repo: string;
    tags?: string[];
  }>,
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

describe("find — no taps configured", () => {
  test("shows no taps message", async () => {
    const { exitCode, stdout } = await runCli(["find"], homeDir, configDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No taps configured");
  });

  test("shows skills.sh results for query even with no taps", async () => {
    const { exitCode, stdout } = await runCli(
      ["find", "react"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("[skills.sh]");
    expect(stdout).toContain("installs");
  });
});

describe("find — with taps", () => {
  test("lists all skills when no query", async () => {
    const tap = await createLocalTap([
      {
        name: "commit-helper",
        description: "Generates commit messages",
        repo: "https://example.com/a",
        tags: ["git"],
      },
      {
        name: "code-review",
        description: "Code review assistant",
        repo: "https://example.com/b",
        tags: ["review"],
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(["find"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("commit-helper");
      expect(stdout).toContain("code-review");
      expect(stdout).toContain("[home]");
    } finally {
      await tap.cleanup();
    }
  });

  test("filters by query", async () => {
    const tap = await createLocalTap([
      {
        name: "commit-helper",
        description: "Generates commit messages",
        repo: "https://example.com/a",
        tags: ["git"],
      },
      {
        name: "code-review",
        description: "Code review assistant",
        repo: "https://example.com/b",
        tags: ["review"],
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["find", "commit"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("commit-helper");
      expect(stdout).not.toContain("code-review");
    } finally {
      await tap.cleanup();
    }
  });

  test("shows no results message when query has no matches", async () => {
    const tap = await createLocalTap([
      {
        name: "commit-helper",
        description: "Commits",
        repo: "https://example.com/a",
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["find", "zzznomatch", "--local"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("No skills found");
    } finally {
      await tap.cleanup();
    }
  });

  test("--json outputs valid JSON", async () => {
    const tap = await createLocalTap([
      {
        name: "commit-helper",
        description: "Generates commit messages",
        repo: "https://example.com/a",
        tags: ["git"],
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["find", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed).toHaveLength(1);
      expect(parsed[0].name).toBe("commit-helper");
      expect(parsed[0].source).toBe("home");
      expect(parsed[0].installRef).toBe("commit-helper");
    } finally {
      await tap.cleanup();
    }
  });

  test("--json with query filter", async () => {
    const tap = await createLocalTap([
      {
        name: "commit-helper",
        description: "Commits",
        repo: "https://example.com/a",
      },
      {
        name: "code-review",
        description: "Reviews",
        repo: "https://example.com/b",
      },
    ]);
    try {
      await runCli(["tap", "add", "home", tap.path], homeDir, configDir);
      const { exitCode, stdout } = await runCli(
        ["find", "commit", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(Array.isArray(parsed)).toBe(true);
      // Tap result for "commit" query should be present
      const tapSkill = parsed.find((e: { source: string; name: string }) => e.source === "home");
      expect(tapSkill).toBeDefined();
      expect(tapSkill.name).toBe("commit-helper");
      // code-review should not appear from the tap (filtered out by query)
      const codeReview = parsed.find(
        (e: { source: string; name: string }) => e.source === "home" && e.name === "code-review",
      );
      expect(codeReview).toBeUndefined();
    } finally {
      await tap.cleanup();
    }
  });
});
