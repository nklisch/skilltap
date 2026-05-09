/**
 * Subprocess tests for `install mcp <source>` (Unit 3.20 of v2.2-cleanup).
 *
 * Coverage:
 *   - happy path: a local repo with `.mcp.json` registers a server, writes
 *     the namespaced entry to state.json, and injects into the agent config.
 *   - smart-scope: outside any git repo lands at global; --scope global is
 *     respected explicitly.
 *   - `mcp:` URL prefix is rejected with a clear hint.
 */

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { loadState } from "@skilltap/core";
import {
  commitAll,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

setDefaultTimeout(60_000);

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

async function disableBuiltinTap(configDir: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), "builtin_tap = false\n");
}

async function createMcpSourceRepo(): Promise<{
  path: string;
  cleanup: () => Promise<void>;
}> {
  const dir = await makeTmpDir();
  await Bun.write(
    join(dir, ".mcp.json"),
    JSON.stringify(
      {
        mcpServers: {
          db: {
            command: "node",
            args: ["server.js"],
          },
        },
      },
      null,
      2,
    ),
  );
  await initRepo(dir);
  await commitAll(dir);
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

describe("install mcp — happy path", () => {
  test("registers server in state.json and injects namespaced entry into agent config", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMcpSourceRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        [
          "install",
          "mcp",
          repo.path,
          "--yes",
          "--scope",
          "global",
          "--also",
          "claude-code",
        ],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Installed");

      const state = await loadState();
      expect(state.ok).toBe(true);
      if (!state.ok) return;
      expect(state.value.mcpServers.length).toBeGreaterThan(0);
      const server = state.value.mcpServers[0]!;
      expect(server.name.startsWith("skilltap:")).toBe(true);
      expect(server.name.endsWith(":db")).toBe(true);

      // Global agent config lives at <SKILLTAP_HOME>/.claude/settings.json
      const settingsPath = join(homeDir, ".claude", "settings.json");
      const settingsRaw = await readFile(settingsPath, "utf8");
      const settings = JSON.parse(settingsRaw) as {
        mcpServers?: Record<string, unknown>;
      };
      expect(settings.mcpServers?.[server.name]).toBeDefined();
    } finally {
      await repo.cleanup();
    }
  });
});

describe("install mcp — smart-scope inference", () => {
  test("from outside any git repo lands at global without explicit --scope", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMcpSourceRepo();
    try {
      // cwd = homeDir → not inside a git repo → smart default = global.
      const { exitCode, stdout } = await runSkilltap(
        ["install", "mcp", repo.path, "--yes", "--also", "claude-code"],
        homeDir,
        configDir,
        homeDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout.toLowerCase()).toContain("global");

      const state = await loadState();
      expect(state.ok).toBe(true);
      if (!state.ok) return;
      expect(state.value.mcpServers.length).toBeGreaterThan(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("--scope global from inside a git repo overrides smart-scope", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMcpSourceRepo();
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      const { exitCode } = await runSkilltap(
        [
          "install",
          "mcp",
          repo.path,
          "--yes",
          "--scope",
          "global",
          "--also",
          "claude-code",
        ],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);

      // Server registered at GLOBAL state.json, not project.
      const globalState = await loadState();
      expect(globalState.ok).toBe(true);
      if (!globalState.ok) return;
      expect(globalState.value.mcpServers.length).toBeGreaterThan(0);

      const projectState = await loadState(projectRoot);
      expect(projectState.ok).toBe(true);
      if (!projectState.ok) return;
      expect(projectState.value.mcpServers.length).toBe(0);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });
});

describe("install mcp — input validation", () => {
  test("rejects the legacy 'mcp:' prefix on user input with a hint", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["install", "mcp", "mcp:github.com/u/r", "--yes", "--scope", "global"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined).toContain("'mcp:' prefix");
  });
});
