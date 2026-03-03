/**
 * Interactive prompt tests — drives @clack/prompts UI flows via a real PTY.
 *
 * Uses a Node.js bridge process (packages/test-utils/src/pty-bridge.mjs) that
 * owns the PTY via node-pty and relays data over JSON-line pipes. This sidesteps
 * Bun's N-API event-loop incompatibility that prevents node-pty onData callbacks
 * from firing when imported directly into a Bun test.
 *
 * Prompt sequence for `install` without --yes / --also:
 *   1. "Install to:"                           (scope select)
 *   2. "Which agents should this skill …?"     (agent symlinks multiselect)
 *   3. [optional] "Which skills to install?"  (multi-skill repos only)
 *   4. "Install <name>?" or "Install N skills?" (confirm)
 *
 * Suppression rules:
 *   --global / --project  → no scope prompt
 *   --yes                 → no agents prompt, no confirm prompt
 *   --also <agent>        → no agents prompt (but confirm still shown)
 *
 * Key sequences:
 *   ENTER   \r   — confirm selection / accept default
 *   SPACE   " "  — toggle item in multiselect
 *   DOWN    \x1b[B — move selection down
 *   CTRL_C  \x03 — cancel / abort
 */
import { lstat } from "node:fs/promises";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  commitAll,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runInteractive,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/..`;
// Relative form — bun resolves src/index.ts against cwd (use only when cwd=CLI_DIR)
const CMD = ["bun", "run", "--bun", "src/index.ts"] as const;
// Absolute form — works regardless of cwd (needed for project-scope tests with custom cwd)
const CMD_ABS = ["bun", "run", "--bun", `${CLI_DIR}/src/index.ts`] as const;

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

function env() {
  return { SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir, DO_NOT_TRACK: "1" };
}

// ---------------------------------------------------------------------------
// install — scope selection prompt
// ---------------------------------------------------------------------------

describe("install — scope prompt", () => {
  test(
    "pressing Enter accepts default scope (Global) and installs",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--skip-scan"],
          { cwd: CLI_DIR, env: env() },
        );

        // 1. Scope prompt
        await session.waitForText("Install to:");
        session.sendKey("ENTER"); // accept Global

        // 2. Agents multiselect
        await session.waitForText("Which agents should this skill");
        session.sendKey("ENTER"); // select none

        // 3. Confirm install
        await session.waitForText("standalone-skill?");
        session.sendKey("ENTER"); // initialValue:true → accepts

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);
        expect(output).toContain("standalone-skill");
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "pressing Down then Enter selects Project scope",
    async () => {
      const repo = await createStandaloneSkillRepo();
      // Need a real git project dir for project-scope installs
      const projectDir = await makeTmpDir();
      try {
        await initRepo(projectDir);
        await Bun.write(`${projectDir}/.gitkeep`, "");
        await commitAll(projectDir, "init");

        // --yes skips agents + confirm so we only need to drive the scope prompt.
        // CMD_ABS uses absolute path so bun can find src/index.ts from projectDir.
        const session = await runInteractive(
          [...CMD_ABS, "install", repo.path, "--skip-scan", "--yes"],
          { cwd: projectDir, env: env() },
        );

        await session.waitForText("Install to:");
        session.sendKey("DOWN"); // Global → Project
        session.sendKey("ENTER");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);
        expect(output).toContain("standalone-skill");
      } finally {
        await repo.cleanup();
        await removeTmpDir(projectDir);
      }
    },
    30_000,
  );

  test(
    "Ctrl+C at scope prompt exits with code 2",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--skip-scan"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Install to:");
        session.sendKey("CTRL_C");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(2);
        expect(output.toLowerCase()).toMatch(/cancel/);
      } finally {
        await repo.cleanup();
      }
    },
    20_000,
  );
});

// ---------------------------------------------------------------------------
// install — agents multiselect
// ---------------------------------------------------------------------------

describe("install — agents prompt", () => {
  test(
    "selecting Claude Code from agents multiselect creates symlink",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan"],
          { cwd: CLI_DIR, env: env() },
        );

        // No scope prompt (--global), go straight to agents
        await session.waitForText("Which agents should this skill");
        session.sendKey("SPACE"); // toggle Claude Code (first item)
        session.sendKey("ENTER");

        // "Save agent selection as default?" follow-up — decline
        await session.waitForText("Save agent selection as default?");
        session.sendKey("ENTER"); // initialValue:false → No

        // Confirm install
        await session.waitForText("standalone-skill?");
        session.sendKey("ENTER");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);

        // Verify symlink exists — use lstat() because Bun.file().exists()
        // returns false for symlinks that point to directories
        const symlinkPath = `${homeDir}/.claude/skills/standalone-skill`;
        const symlinkExists = await lstat(symlinkPath).then(() => true).catch(() => false);
        expect(symlinkExists).toBe(true);
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "pressing Enter with none selected skips symlinks",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Which agents should this skill");
        session.sendKey("ENTER"); // none selected, required:false

        await session.waitForText("standalone-skill?");
        session.sendKey("ENTER");

        const { exitCode } = await session.finish();
        expect(exitCode).toBe(0);
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );
});

// ---------------------------------------------------------------------------
// install — confirm prompt
// ---------------------------------------------------------------------------

describe("install — confirm prompt", () => {
  test(
    "Enter confirms install (initialValue:true)",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          // --also skips agents prompt → scope then confirm
          [...CMD, "install", repo.path, "--global", "--skip-scan", "--also", "claude-code"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("standalone-skill?");
        session.sendKey("ENTER"); // accept default yes

        const { exitCode } = await session.finish();
        expect(exitCode).toBe(0);
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "Ctrl+C at confirm prompt exits with code 2",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan", "--also", "claude-code"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("standalone-skill?");
        session.sendKey("CTRL_C");

        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await repo.cleanup();
      }
    },
    20_000,
  );
});

// ---------------------------------------------------------------------------
// install — multi-skill selection
// ---------------------------------------------------------------------------

describe("install — skill selection (multi-skill repo)", () => {
  test(
    "Space selects first skill only — only that skill installed",
    async () => {
      const repo = await createMultiSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan", "--also", "claude-code"],
          { cwd: CLI_DIR, env: env() },
        );

        // Multiselect: nothing pre-selected, required:true
        await session.waitForText("Which skills to install?");
        session.sendKey("SPACE"); // select skill-a (first item)
        session.sendKey("ENTER");

        // Confirm
        await session.waitForText("Install");
        session.sendKey("ENTER");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);
        expect(output).toContain("skill-a");
        expect(output).not.toContain("skill-b installed");
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "selecting both skills with Space+Down+Space installs both",
    async () => {
      const repo = await createMultiSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan", "--also", "claude-code"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Which skills to install?");
        session.sendKey("SPACE"); // select skill-a
        session.sendKey("DOWN");
        session.sendKey("SPACE"); // select skill-b
        session.sendKey("ENTER");

        await session.waitForText("Install");
        session.sendKey("ENTER");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);
        expect(output).toContain("skill-a");
        expect(output).toContain("skill-b");
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "Ctrl+C at skill selection exits with code 2",
    async () => {
      const repo = await createMultiSkillRepo();
      try {
        const session = await runInteractive(
          [...CMD, "install", repo.path, "--global", "--skip-scan", "--also", "claude-code"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Which skills to install?");
        session.sendKey("CTRL_C");

        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await repo.cleanup();
      }
    },
    20_000,
  );
});

// ---------------------------------------------------------------------------
// remove — confirm prompt
// ---------------------------------------------------------------------------

describe("remove — confirm prompt", () => {
  async function installSkill(repoPath: string) {
    // --yes --also skips agents and confirm prompts
    const session = await runInteractive(
      [...CMD, "install", repoPath, "--global", "--skip-scan", "--yes", "--also", "claude-code"],
      { cwd: CLI_DIR, env: env() },
    );
    const { exitCode } = await session.finish();
    if (exitCode !== 0) throw new Error("Setup: install failed");
  }

  test(
    "pressing y + Enter removes the skill",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        await installSkill(repo.path);

        const session = await runInteractive(
          [...CMD, "remove", "standalone-skill"],
          { cwd: CLI_DIR, env: env() },
        );

        // confirm prompt — initialValue:false so we must explicitly press y
        await session.waitForText("Remove standalone-skill?");
        session.send("y");
        session.sendKey("ENTER");

        const { exitCode, output } = await session.finish();
        expect(exitCode).toBe(0);
        expect(output.toLowerCase()).toMatch(/remov/);
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );

  test(
    "pressing Enter with default (No) aborts removal",
    async () => {
      const repo = await createStandaloneSkillRepo();
      try {
        await installSkill(repo.path);

        const session = await runInteractive(
          [...CMD, "remove", "standalone-skill"],
          { cwd: CLI_DIR, env: env() },
        );

        // initialValue:false — ENTER selects "No"
        await session.waitForText("Remove standalone-skill?");
        session.sendKey("ENTER");

        const { exitCode } = await session.finish();
        expect(exitCode).not.toBe(0); // cancelled
      } finally {
        await repo.cleanup();
      }
    },
    30_000,
  );
});
