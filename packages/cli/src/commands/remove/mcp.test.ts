/**
 * Subprocess tests for `remove mcp <source>` (Unit 3.20 of v2.2-cleanup).
 *
 * Coverage:
 *   - removing an installed MCP server clears the namespaced key from the
 *     agent settings file and drops the record from state.json.
 *   - unknown source name exits non-zero with a hint.
 *   - the legacy `mcp:` URL prefix on user input is rejected.
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
          db: { command: "node", args: ["server.js"] },
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

describe("remove mcp — happy path", () => {
  test("clears the namespaced key from agent settings and drops the state record", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMcpSourceRepo();
    try {
      const install = await runSkilltap(
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
      expect(install.exitCode).toBe(0);

      // Sanity: server present in state and settings.
      const before = await loadState();
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      expect(before.value.mcpServers.length).toBeGreaterThan(0);
      const namespacedKey = before.value.mcpServers[0]!.name;
      expect(namespacedKey.startsWith("skilltap:")).toBe(true);

      const settingsPath = join(homeDir, ".claude", "settings.json");
      const settingsBefore = JSON.parse(
        await readFile(settingsPath, "utf8"),
      ) as { mcpServers?: Record<string, unknown> };
      expect(settingsBefore.mcpServers?.[namespacedKey]).toBeDefined();

      // Remove.
      const remove = await runSkilltap(
        ["remove", "mcp", repo.path, "--yes", "--scope", "global"],
        homeDir,
        configDir,
      );
      expect(remove.exitCode).toBe(0);
      expect(remove.stdout).toContain("Removed");

      // State entry gone.
      const after = await loadState();
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      expect(after.value.mcpServers.find((m) => m.name === namespacedKey)).toBeUndefined();

      // Namespaced key gone from agent settings.
      const settingsAfter = JSON.parse(
        await readFile(settingsPath, "utf8"),
      ) as { mcpServers?: Record<string, unknown> };
      expect(settingsAfter.mcpServers?.[namespacedKey]).toBeUndefined();
    } finally {
      await repo.cleanup();
    }
  });
});

describe("remove mcp — error paths", () => {
  test("removing an unknown source exits non-zero with a hint", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["remove", "mcp", "/no/such/path", "--yes", "--scope", "global"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined.length).toBeGreaterThan(0);
  });

  test("rejects the legacy 'mcp:' prefix on user input", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["remove", "mcp", "mcp:github.com/u/r", "--yes", "--scope", "global"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined).toContain("'mcp:' prefix");
  });
});
