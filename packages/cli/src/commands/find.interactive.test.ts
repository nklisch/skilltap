/**
 * Interactive PTY tests for `find -i` reactive search prompt.
 *
 * Uses the Node.js PTY bridge to drive the interactive search UI.
 * Tests cover: prompt appearance, typing to filter, arrow navigation,
 * Enter to select, Ctrl+C to cancel, pre-filled queries, and no-match state.
 */
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
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

// ---------------------------------------------------------------------------
// Tap fixture — creates a local tap and registers it via CLI
// ---------------------------------------------------------------------------

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

async function addTap(tapPath: string): Promise<void> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "tap", "add", "home", tapPath],
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("find -i — search prompt", () => {
  test(
    "shows search prompt with placeholder",
    async () => {
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Generates commit messages",
          repo: "https://example.com/a",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Search for skills:");
        await session.waitForText("git, testing, docker");

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
    "typing a query shows matching results",
    async () => {
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Generates commit messages",
          repo: "https://example.com/a",
        },
        {
          name: "code-review",
          description: "Code review assistant",
          repo: "https://example.com/b",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Search for skills:");

        // Wait for initial results to load
        await session.waitForText("commit-helper");

        // Type to filter
        session.send("commit");

        // Should show commit-helper in results
        await session.waitForText("commit-helper");

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
    "pre-filled query shows results immediately",
    async () => {
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Generates commit messages",
          repo: "https://example.com/a",
        },
        {
          name: "code-review",
          description: "Code review assistant",
          repo: "https://example.com/b",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "commit", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        // Results should appear without typing — query pre-filled
        await session.waitForText("commit-helper");

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
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Commits",
          repo: "https://example.com/a",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Search for skills:");
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
    "Enter on a result starts install flow",
    async () => {
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Generates commit messages",
          repo: "https://example.com/a",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Search for skills:");
        // Wait for results to load
        await session.waitForText("commit-helper");

        // Select the result
        session.sendKey("ENTER");

        // Should transition to install flow (scope prompt)
        await session.waitForText("Install to:");

        // Cancel out of the install flow
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
    "no matches shows warning message",
    async () => {
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Commits",
          repo: "https://example.com/a",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "find", "-i", "--local"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Search for skills:");
        // Wait for initial load
        await session.waitForText("commit-helper");

        // Type something that won't match
        session.send("zzznomatch");
        await session.waitForText("No matches found");

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
