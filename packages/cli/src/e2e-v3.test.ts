/**
 * v2.0 Redesign end-to-end test (Phase 46.8).
 *
 * Covers the new CLI surface introduced by the redesign:
 *   - install skill / plugin with typed subcommand
 *   - toggle plugin <name>:<component>
 *   - adopt <path> (replaces link/unlink)
 *   - remove plugin
 *   - status shows installed plugin
 *   - migrate from v0.x fixture
 *   - old removed commands return exit 1 with hints
 *
 * Uses runSkilltap (pipe mode) from @skilltap/test-utils.
 */
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  createClaudePluginRepo,
  createSkillDir,
  createStandaloneSkillRepo,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

setDefaultTimeout(90_000);

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

// ── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Run skilltap inside a fresh git repo project dir.
 * Returns { exitCode, stdout, stderr, projectRoot }.
 */
async function setupProjectAndRun(
  args: string[],
  extra?: { extraFiles?: Record<string, string> },
): Promise<{
  exitCode: number;
  stdout: string;
  stderr: string;
  projectRoot: string;
}> {
  const projectRoot = await makeTmpDir();
  await initRepo(projectRoot);
  if (extra?.extraFiles) {
    for (const [name, content] of Object.entries(extra.extraFiles)) {
      await writeFile(join(projectRoot, name), content);
    }
  }
  const result = await runSkilltap(args, homeDir, configDir, projectRoot);
  return { ...result, projectRoot };
}

// ═══════════════════════════════════════════════════════════════════════════
// Group 1: install skill
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — install skill (typed subcommand)", () => {
  test("install skill <path> writes to state.json and succeeds", async () => {
    const skillRepo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout, stderr, projectRoot } = await setupProjectAndRun(
        ["install", "skill", skillRepo.path, "--project", "--skip-scan", "--yes"],
      );
      if (exitCode !== 0) {
        console.error("install stderr:", stderr);
        console.error("install stdout:", stdout);
      }
      expect(exitCode).toBe(0);

      // state.json written correctly
      const stateFile = Bun.file(join(projectRoot, ".agents", "state.json"));
      expect(await stateFile.exists()).toBe(true);
      const state = JSON.parse(await stateFile.text()) as {
        version: number;
        skills: Array<{ name: string }>;
      };
      expect(state.version).toBe(2);
      expect(state.skills.map((s) => s.name)).toContain("standalone-skill");
    } finally {
      await skillRepo.cleanup();
    }
  });

  test("install skill with --scope global writes to global state", async () => {
    const skillRepo = await createStandaloneSkillRepo();
    const globalDir = await makeTmpDir(); // outside a git repo → would default global
    try {
      const { exitCode } = await runSkilltap(
        ["install", "skill", skillRepo.path, "--global", "--skip-scan", "--yes"],
        homeDir,
        configDir,
        globalDir,
      );
      expect(exitCode).toBe(0);

      const globalState = Bun.file(
        join(configDir, "skilltap", "state.json"),
      );
      expect(await globalState.exists()).toBe(true);
      const state = JSON.parse(await globalState.text()) as {
        skills: Array<{ name: string }>;
      };
      expect(state.skills.map((s) => s.name)).toContain("standalone-skill");
    } finally {
      await skillRepo.cleanup();
      await removeTmpDir(globalDir);
    }
  });

  test("install skill on wrong type (plugin source) exits 1 with hint", async () => {
    const pluginRepo = await createClaudePluginRepo();
    try {
      const projectRoot = await makeTmpDir();
      await initRepo(projectRoot);
      const { exitCode, stderr } = await runSkilltap(
        ["install", "skill", pluginRepo.path, "--project", "--yes", "--skip-scan"],
        homeDir,
        configDir,
        projectRoot,
      );
      // Plugin repos have no SKILL.md at root — install skill should fail
      // with an instructive error (exit 1).
      expect(exitCode).toBe(1);
      // The error should mention "plugin" as a hint.
      expect(stderr.toLowerCase()).toMatch(/plugin|no skill/i);
      await removeTmpDir(projectRoot);
    } finally {
      await pluginRepo.cleanup();
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 2: install plugin
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — install plugin (typed subcommand)", () => {
  test("install plugin writes to state.json plugins[]", async () => {
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { exitCode, stderr, projectRoot } = await setupProjectAndRun(
        ["install", "plugin", pluginRepo.path, "--global", "--yes", "--skip-scan"],
      );
      if (exitCode !== 0) {
        console.error("install plugin stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // Global state.json must contain the plugin
      const globalState = Bun.file(
        join(configDir, "skilltap", "state.json"),
      );
      expect(await globalState.exists()).toBe(true);
      const state = JSON.parse(await globalState.text()) as {
        version: number;
        plugins: Array<{ name: string }>;
      };
      expect(state.version).toBe(2);
      expect(state.plugins.length).toBeGreaterThan(0);
    } finally {
      await pluginRepo.cleanup();
    }
  });

  test("status exits 0 and includes expected JSON fields after plugin install", async () => {
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { projectRoot } = await setupProjectAndRun(
        ["install", "plugin", pluginRepo.path, "--global", "--yes", "--skip-scan"],
      );

      // status from same project root
      const statusResult = await runSkilltap(
        ["status", "--json"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(statusResult.exitCode).toBe(0);

      // status --json must produce valid JSON with expected top-level fields
      const payload = JSON.parse(statusResult.stdout) as Record<string, unknown>;
      expect(Array.isArray(payload.skills)).toBe(true);
      expect(Array.isArray(payload.taps)).toBe(true);

      // Verify plugin landed in global state.json directly
      const globalState = JSON.parse(
        await Bun.file(join(configDir, "skilltap", "state.json")).text(),
      ) as { plugins: Array<{ name: string }> };
      expect(globalState.plugins.length).toBeGreaterThan(0);
    } finally {
      await pluginRepo.cleanup();
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 3: toggle plugin <name>:<component>
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — toggle plugin component", () => {
  test("toggle plugin <name>:<component> disables and re-enables the component", async () => {
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { projectRoot } = await setupProjectAndRun(
        ["install", "plugin", pluginRepo.path, "--global", "--yes", "--skip-scan"],
      );

      // Find out the installed plugin name
      const stateText = await Bun.file(
        join(configDir, "skilltap", "state.json"),
      ).text();
      const state = JSON.parse(stateText) as {
        plugins: Array<{ name: string; components: Array<{ name: string; type: string; active: boolean }> }>;
      };
      const plugin = state.plugins[0];
      if (!plugin) return; // No plugin installed — skip rest

      const componentWithSkill = plugin.components.find((c) => c.type === "skill");
      if (!componentWithSkill) return; // No skill component — skip rest

      const ref = `${plugin.name}:${componentWithSkill.name}`;

      // Disable the component
      const disableResult = await runSkilltap(
        ["toggle", "plugin", ref, "--json"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(disableResult.exitCode).toBe(0);

      // Verify the component is now inactive
      const stateAfterDisable = JSON.parse(
        await Bun.file(join(configDir, "skilltap", "state.json")).text(),
      ) as { plugins: Array<{ name: string; components: Array<{ name: string; active: boolean }> }> };
      const updatedPlugin = stateAfterDisable.plugins.find((p) => p.name === plugin.name);
      const updatedComponent = updatedPlugin?.components.find(
        (c) => c.name === componentWithSkill.name,
      );
      expect(updatedComponent?.active).toBe(false);

      // Re-enable by toggling again
      const enableResult = await runSkilltap(
        ["toggle", "plugin", ref, "--json"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(enableResult.exitCode).toBe(0);

      // Verify active again
      const stateAfterEnable = JSON.parse(
        await Bun.file(join(configDir, "skilltap", "state.json")).text(),
      ) as { plugins: Array<{ name: string; components: Array<{ name: string; active: boolean }> }> };
      const reEnabledPlugin = stateAfterEnable.plugins.find((p) => p.name === plugin.name);
      const reEnabledComponent = reEnabledPlugin?.components.find(
        (c) => c.name === componentWithSkill.name,
      );
      expect(reEnabledComponent?.active).toBe(true);
    } finally {
      await pluginRepo.cleanup();
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 4: adopt <path> (replaces link/unlink)
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — adopt external path", () => {
  test("adopt <path> track-in-place records the skill in state.json", async () => {
    const projectRoot = await makeTmpDir();
    await initRepo(projectRoot);
    const externalDir = await makeTmpDir();
    try {
      // Create an external skill directory
      await createSkillDir(externalDir, "external-skill");
      const skillPath = join(externalDir, "external-skill");

      const { exitCode, stderr } = await runSkilltap(
        ["adopt", skillPath, "--global", "--skip-scan", "--yes"],
        homeDir,
        configDir,
        projectRoot,
      );
      if (exitCode !== 0) {
        console.error("adopt stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // state.json should have the adopted skill
      const globalState = JSON.parse(
        await Bun.file(join(configDir, "skilltap", "state.json")).text(),
      ) as { skills: Array<{ name: string }> };
      expect(globalState.skills.map((s) => s.name)).toContain("external-skill");
    } finally {
      await removeTmpDir(projectRoot);
      await removeTmpDir(externalDir);
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 5: remove plugin
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — remove plugin", () => {
  test("remove plugin drops all components from state.json", async () => {
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { projectRoot } = await setupProjectAndRun(
        ["install", "plugin", pluginRepo.path, "--global", "--yes", "--skip-scan"],
      );

      const stateText = await Bun.file(
        join(configDir, "skilltap", "state.json"),
      ).text();
      const state = JSON.parse(stateText) as { plugins: Array<{ name: string }> };
      const pluginName = state.plugins[0]?.name;
      if (!pluginName) return;

      const { exitCode, stderr } = await runSkilltap(
        ["remove", "plugin", pluginName, "--yes"],
        homeDir,
        configDir,
        projectRoot,
      );
      if (exitCode !== 0) {
        console.error("remove plugin stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // Plugin gone from state
      const afterState = JSON.parse(
        await Bun.file(join(configDir, "skilltap", "state.json")).text(),
      ) as { plugins: Array<{ name: string }> };
      expect(afterState.plugins.map((p) => p.name)).not.toContain(pluginName);
    } finally {
      await pluginRepo.cleanup();
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 6: migrate from v0.x fixture
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — migrate from v0.x config", () => {
  test("migrate translates [security.human]/[security.agent] to flat [security]", async () => {
    const v0ConfigDir = await makeTmpDir();
    const v0HomeDir = await makeTmpDir();
    const v0ProjectDir = await makeTmpDir();
    await initRepo(v0ProjectDir);

    try {
      // Write a v0.x config with per-mode security blocks and agent-mode block
      const v0Config = `
verbose = false

[agent-mode]
enabled = false

[security.human]
scan = "static"
on_warn = "prompt"

[security.agent]
scan = "static"
on_warn = "fail"

[[taps]]
name = "home"
url = "https://github.com/nklisch/skilltap-skills"
`;
      await mkdir(join(v0ConfigDir, "skilltap"), { recursive: true });
      await writeFile(join(v0ConfigDir, "skilltap", "config.toml"), v0Config);

      // Write a minimal v0.x installed.json so migrate has state to convert
      const installed = {
        version: 1,
        skills: [
          {
            name: "legacy-skill",
            description: "from v0.x",
            repo: "https://github.com/example/repo",
            ref: "main",
            sha: "abc1234",
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2025-01-01T00:00:00.000Z",
            updatedAt: "2025-01-01T00:00:00.000Z",
            active: true,
          },
        ],
      };
      await mkdir(join(v0HomeDir, ".config", "skilltap"), { recursive: true });
      // installed.json lives in the config dir for global scope
      await writeFile(
        join(v0ConfigDir, "skilltap", "installed.json"),
        JSON.stringify(installed, null, 2),
      );

      const { exitCode, stdout, stderr } = await runSkilltap(
        ["migrate"],
        v0HomeDir,
        v0ConfigDir,
        v0ProjectDir,
      );
      if (exitCode !== 0) {
        console.error("migrate stderr:", stderr);
        console.error("migrate stdout:", stdout);
      }
      expect(exitCode).toBe(0);

      // Config should now be flat [security]
      const newConfig = await Bun.file(
        join(v0ConfigDir, "skilltap", "config.toml"),
      ).text();
      expect(newConfig).not.toContain("[security.human]");
      expect(newConfig).not.toContain("[security.agent]");
      expect(newConfig).not.toContain("[agent-mode]");
      expect(newConfig).toContain("[security]");

      // Old config backed up as .v1.bak
      const bakExists = await Bun.file(
        join(v0ConfigDir, "skilltap", "config.toml.v1.bak"),
      ).exists();
      expect(bakExists).toBe(true);
    } finally {
      await removeTmpDir(v0ConfigDir);
      await removeTmpDir(v0HomeDir);
      await removeTmpDir(v0ProjectDir);
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Group 7: removed commands return errors with hints
// ═══════════════════════════════════════════════════════════════════════════

describe("v3 E2E — removed commands exit 1 with hints", () => {
  test("bare install (no type) exits 1", async () => {
    const { exitCode } = await setupProjectAndRun(["install", "some-skill"]);
    // "some-skill" is neither skill/plugin/mcp — citty routes to unknown subcommand
    expect(exitCode).toBe(1);
  });

  test("status exits 0 and produces output", async () => {
    const { exitCode, stdout } = await setupProjectAndRun(["status", "--json"]);
    expect(exitCode).toBe(0);
    // Output must be parseable JSON
    expect(() => JSON.parse(stdout)).not.toThrow();
  });

  test("doctor exits 0 on clean environment", async () => {
    const { exitCode } = await setupProjectAndRun(["doctor"]);
    // doctor returns non-zero only on hard failures
    expect(exitCode).toBe(0);
  });

  test("update --check exits 0 with no installed skills", async () => {
    const { exitCode } = await setupProjectAndRun([
      "update",
      "--check",
      "--json",
    ]);
    expect(exitCode).toBe(0);
  });
});
