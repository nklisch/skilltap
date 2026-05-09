/**
 * Subprocess tests for the `toggle` command (Unit 3.20 of v2.2-cleanup).
 *
 * Coverage:
 *   - `toggle skill <name>` flips the active flag and re-flips on second call.
 *   - `toggle plugin <name>:<component>` toggles a single component.
 *   - `toggle mcp <name>` reports the not-yet-implemented hint without crashing.
 *   - bare `toggle` in non-TTY mode errors with usage hint and exits 1.
 *   - invalid type (e.g. `toggle widget foo`) errors with valid-types list.
 *   - unknown skill name errors with "not installed" hint.
 *
 * All tests run in project scope — they create a git repo as the cwd so
 * `tryFindProjectRoot` resolves to that repo and the install state lands at
 * `<repo>/.agents/state.json` (matching where toggle reads from).
 */

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { loadState } from "@skilltap/core";
import {
  createClaudePluginRepo,
  createStandaloneSkillRepo,
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
let projectRoot: string;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
  projectRoot = await makeTmpDir();
  await initRepo(projectRoot);
});

afterEach(async () => {
  await env.cleanup();
  await removeTmpDir(projectRoot);
});

async function disableBuiltinTap(configDir: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), "builtin_tap = false\n");
}

describe("toggle skill", () => {
  test("flips the active flag on a known skill, then back", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    try {
      const install = await runSkilltap(
        [
          "install",
          "skill",
          repo.path,
          "--yes",
          "--scope",
          "project",
          "--skip-scan",
        ],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(install.exitCode).toBe(0);

      const first = await runSkilltap(
        ["toggle", "skill", "standalone-skill"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(first.exitCode).toBe(0);
      expect(first.stdout.toLowerCase()).toContain("disabled");

      const stateAfterDisable = await loadState(projectRoot);
      expect(stateAfterDisable.ok).toBe(true);
      if (!stateAfterDisable.ok) return;
      const skill = stateAfterDisable.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(skill).toBeDefined();
      expect(skill!.active).toBe(false);

      // Toggle again — should re-enable.
      const second = await runSkilltap(
        ["toggle", "skill", "standalone-skill"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(second.exitCode).toBe(0);
      expect(second.stdout.toLowerCase()).toContain("enabled");

      const stateAfterEnable = await loadState(projectRoot);
      expect(stateAfterEnable.ok).toBe(true);
      if (!stateAfterEnable.ok) return;
      const skillBack = stateAfterEnable.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(skillBack!.active !== false).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("unknown skill name exits 1 with hint", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle", "skill", "no-such-skill"],
      homeDir,
      configDir,
      projectRoot,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined.toLowerCase()).toContain("not installed");
  });
});

describe("toggle plugin", () => {
  test("plugin name:component toggles a single component", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createClaudePluginRepo();
    try {
      const install = await runSkilltap(
        [
          "install",
          "plugin",
          repo.path,
          "--yes",
          "--scope",
          "project",
          "--skip-scan",
        ],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(install.exitCode).toBe(0);

      const before = await loadState(projectRoot);
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      const plugin = before.value.plugins.find((p) => p.name === "test-plugin");
      expect(plugin).toBeDefined();
      const firstComponent = plugin!.components[0];
      expect(firstComponent).toBeDefined();
      const componentName = firstComponent!.name;
      const wasActive = firstComponent!.active !== false;

      const { exitCode, stdout } = await runSkilltap(
        ["toggle", "plugin", `test-plugin:${componentName}`],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      const expectedAction = wasActive ? "disabled" : "enabled";
      expect(stdout.toLowerCase()).toContain(expectedAction);

      const after = await loadState(projectRoot);
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      const pluginAfter = after.value.plugins.find(
        (p) => p.name === "test-plugin",
      );
      const componentAfter = pluginAfter!.components.find(
        (c) => c.name === componentName,
      );
      expect(componentAfter).toBeDefined();
      expect(componentAfter!.active).toBe(!wasActive);
    } finally {
      await repo.cleanup();
    }
  });

  test("unknown component name returns ambiguity / not-found error", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createClaudePluginRepo();
    try {
      await runSkilltap(
        [
          "install",
          "plugin",
          repo.path,
          "--yes",
          "--scope",
          "project",
          "--skip-scan",
        ],
        homeDir,
        configDir,
        projectRoot,
      );

      const { exitCode, stdout, stderr } = await runSkilltap(
        ["toggle", "plugin", "test-plugin:no-such-component"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(1);
      const combined = stdout + stderr;
      expect(combined.toLowerCase()).toMatch(/not found|available/);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("toggle mcp", () => {
  test("reports the not-yet-implemented hint when toggling a registered MCP", async () => {
    // Hand-write project state.json with a registered MCP server so toggle
    // reaches the "not implemented" branch instead of the unknown-name branch.
    const agentsDir = join(projectRoot, ".agents");
    await mkdir(agentsDir, { recursive: true });
    await Bun.write(
      join(agentsDir, "state.json"),
      JSON.stringify(
        {
          version: 2,
          skills: [],
          plugins: [],
          mcpServers: [
            {
              name: "skilltap:test-mcp",
              source: "https://example.test/mcp",
              config: {
                type: "stdio",
                command: "echo",
                args: [],
                env: {},
              },
              targets: [],
              installedAt: new Date().toISOString(),
            },
          ],
        },
        null,
        2,
      ),
    );

    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle", "mcp", "skilltap:test-mcp"],
      homeDir,
      configDir,
      projectRoot,
    );

    expect(exitCode).toBe(0);
    const combined = stdout + stderr;
    expect(combined.toLowerCase()).toMatch(/not.*implemented|remove/);
  });

  test("unknown MCP server name exits 1 with hint", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle", "mcp", "no-such-mcp"],
      homeDir,
      configDir,
      projectRoot,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined.toLowerCase()).toContain("not installed");
  });
});

describe("toggle (bare invocation)", () => {
  test("non-TTY bare invocation exits 1 with usage hint", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle"],
      homeDir,
      configDir,
      projectRoot,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined).toContain("non-interactive");
    expect(combined.toLowerCase()).toContain("usage");
  });

  test("missing target with type given exits 1", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle", "skill"],
      homeDir,
      configDir,
      projectRoot,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined.toLowerCase()).toContain("type and target");
  });

  test("invalid type errors with valid-types list", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["toggle", "widget", "foo"],
      homeDir,
      configDir,
      projectRoot,
    );
    expect(exitCode).toBe(1);
    const combined = stdout + stderr;
    expect(combined).toContain("Invalid type");
    expect(combined).toContain("skill");
    expect(combined).toContain("plugin");
    expect(combined).toContain("mcp");
  });
});
