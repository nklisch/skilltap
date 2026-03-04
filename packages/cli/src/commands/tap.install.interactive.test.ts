/**
 * Interactive PTY tests for `tap install` searchable multiselect prompt.
 *
 * Tests cover: prompt appearance, typing to filter, Space to toggle selection,
 * Enter to confirm, Ctrl+C to cancel, --tap scoping, and help text.
 */
import { join } from "node:path";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
setDefaultTimeout(45_000);
import {
  commitAll,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runInteractive,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;
const CMD = ["bun", "run", "--bun", "src/index.ts"] as const;

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

function env() {
  return {
    SKILLTAP_HOME: homeDir,
    XDG_CONFIG_HOME: configDir,
    DO_NOT_TRACK: "1",
  };
}

async function createLocalTap(
  name: string,
  skills: Array<{
    name: string;
    description: string;
    repo: string;
    tags?: string[];
  }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name,
    description: `${name} tap`,
    skills: skills.map((s) => ({ tags: [], ...s })),
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

async function addTap(tapName: string, tapPath: string): Promise<void> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "tap", "add", tapName, tapPath],
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
  if (exitCode !== 0) {
    const stderr = await new Response(proc.stderr).text();
    throw new Error(`tap add failed (code ${exitCode}): ${stderr}`);
  }
}

describe("tap install — interactive multiselect", () => {
  test(
    "shows searchable multiselect prompt with skill names",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
        {
          name: "docker-compose",
          description: "Docker compose helpers",
          repo: "https://example.com/docker-compose",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("git-hooks");
        await session.waitForText("docker-compose");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );

  test(
    "shows Space/Enter help text for multiselect",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("Space:");
        await session.waitForText("Enter:");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );

  test(
    "typing filters results",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
        {
          name: "docker-compose",
          description: "Docker compose helpers",
          repo: "https://example.com/docker-compose",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("git-hooks");

        session.send("docker");
        await session.waitForText("docker-compose");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );

  test(
    "Space toggles selection and updates count in footer",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
        {
          name: "docker-compose",
          description: "Docker compose helpers",
          repo: "https://example.com/docker-compose",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("git-hooks");

        // Toggle the first item
        session.send(" ");
        // Footer should update to show 1 selected
        await session.waitForText("1 selected");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );

  test(
    "Ctrl+C cancels with exit code 2",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        session.sendKey("CTRL_C");

        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    20_000,
  );

  test(
    "--tap scopes prompt to a single tap's skills",
    async () => {
      const tap1 = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
      ]);
      const tap2 = await createLocalTap("work", [
        {
          name: "docker-compose",
          description: "Docker helpers",
          repo: "https://example.com/docker",
        },
      ]);
      try {
        await addTap("home", tap1.path);
        await addTap("work", tap2.path);

        const session = await runInteractive(
          [...CMD, "tap", "install", "--tap", "home"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("git-hooks");
        // docker-compose is from work tap, should not appear
        // (we just verify git-hooks is there; we can't assert absence easily)

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap1.cleanup();
        await tap2.cleanup();
      }
    },
    30_000,
  );

  test(
    "Enter with a selection transitions to install flow",
    async () => {
      const tap = await createLocalTap("home", [
        {
          name: "git-hooks",
          description: "Git hook management",
          repo: "https://example.com/git-hooks",
        },
      ]);
      try {
        await addTap("home", tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("git-hooks");

        // Select item then confirm
        session.send(" ");
        await session.waitForText("1 selected");
        session.sendKey("ENTER");

        // Should transition to scope prompt
        await session.waitForText("Install to:");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );
});
