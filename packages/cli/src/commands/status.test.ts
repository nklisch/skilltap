/**
 * Subprocess tests for the `status` command.
 *
 * Coverage:
 *   - default render shows skills + plugins + taps sections.
 *   - --json output is valid JSON with the expected shape.
 *   - --disabled filter limits to inactive items.
 *   - --active filter limits to active items.
 *   - --scope global/project filters apply to skills and plugins.
 *   - --unmanaged switches to discovery view.
 *   - smart-scope inference: outside a git repo → global view; inside → project.
 *   - drift summary line surfaces when manifest disagrees with state.
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
import {
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

describe("status — default render", () => {
  test("renders header, scope, targets and section labels", async () => {
    const { exitCode, stdout } = await runSkilltap(
      ["status"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("skilltap status");
    expect(stdout).toContain("Scope:");
    expect(stdout).toContain("Targets:");
    // Empty environment → all three sections render the (none) marker.
    expect(stdout).toContain("Skills:");
    expect(stdout).toContain("Plugins:");
    expect(stdout).toContain("Taps");
  });

  test("includes built-in tap row when enabled", async () => {
    const { stdout } = await runSkilltap(["status"], homeDir, configDir);
    expect(stdout).toContain("(built-in)");
  });
});

describe("status --json", () => {
  test("emits valid JSON with all top-level fields", async () => {
    const { exitCode, stdout } = await runSkilltap(
      ["status", "--json"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    let parsed: unknown;
    expect(() => {
      parsed = JSON.parse(stdout);
    }).not.toThrow();
    const report = parsed as Record<string, unknown>;
    expect(Array.isArray(report.skills)).toBe(true);
    expect(Array.isArray(report.plugins)).toBe(true);
    expect(Array.isArray(report.taps)).toBe(true);
    expect(typeof report.scope).toBe("string");
    expect(report).toHaveProperty("hasManifest");
    expect(report).toHaveProperty("fromV2State");
  });

  test("--json --global limits skills to global scope only", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      // Install once at global, once at project.
      await runSkilltap(
        [
          "install",
          "skill",
          repo.path,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
        projectRoot,
      );
      await runSkilltap(
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

      const { stdout } = await runSkilltap(
        ["status", "--json", "--scope", "global"],
        homeDir,
        configDir,
        projectRoot,
      );
      const report = JSON.parse(stdout) as {
        skills: Array<{ name: string; scope: string }>;
      };
      // The status report's scope inference will be "project" (we're inside a
      // git repo), but with --global filter applied the skills array should
      // ONLY include global-scoped records. With smart-scope inference today,
      // the report.skills come from the inferred scope's state file. Either
      // way, every returned skill must have scope === "global".
      for (const s of report.skills) {
        expect(s.scope).toBe("global");
      }
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });
});

describe("status --disabled / --active filters", () => {
  test("--disabled filters to disabled skills only", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      await runSkilltap(
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

      // All skills are active by default — --disabled filter should hide all.
      const activeRun = await runSkilltap(
        ["status", "--json", "--disabled"],
        homeDir,
        configDir,
        projectRoot,
      );
      const activeReport = JSON.parse(activeRun.stdout) as {
        skills: Array<{ name: string }>;
      };
      expect(activeReport.skills).toHaveLength(0);

      // Disable the skill, then --disabled filter should show it.
      await runSkilltap(
        ["toggle", "skill", "standalone-skill"],
        homeDir,
        configDir,
        projectRoot,
      );
      const disabledRun = await runSkilltap(
        ["status", "--json", "--disabled"],
        homeDir,
        configDir,
        projectRoot,
      );
      const disabledReport = JSON.parse(disabledRun.stdout) as {
        skills: Array<{ name: string; active: boolean }>;
      };
      expect(disabledReport.skills.length).toBe(1);
      expect(disabledReport.skills[0]?.name).toBe("standalone-skill");
      expect(disabledReport.skills[0]?.active).toBe(false);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("--active filters to active skills only", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      await runSkilltap(
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

      const { stdout } = await runSkilltap(
        ["status", "--json", "--active"],
        homeDir,
        configDir,
        projectRoot,
      );
      const report = JSON.parse(stdout) as {
        skills: Array<{ name: string; active: boolean }>;
      };
      expect(report.skills.length).toBe(1);
      expect(report.skills[0]?.active).toBe(true);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });
});

describe("status --unmanaged", () => {
  test("renders a separate Unmanaged section header (or empty info line)", async () => {
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      const { exitCode, stdout } = await runSkilltap(
        ["status", "--unmanaged"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      // Either we see the bold "Unmanaged skills" header (skills present) or
      // the empty info line. Both are valid for a fresh project.
      expect(stdout).toMatch(/Unmanaged skills|No unmanaged skills/);
    } finally {
      await removeTmpDir(projectRoot);
    }
  });

  test("--unmanaged --json returns an array", async () => {
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      const { exitCode, stdout } = await runSkilltap(
        ["status", "--unmanaged", "--json"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(Array.isArray(parsed)).toBe(true);
    } finally {
      await removeTmpDir(projectRoot);
    }
  });
});

describe("status — smart-scope inference reporting", () => {
  test("outside any git repo, scope is reported as global", async () => {
    // Use the homeDir as cwd — it is not inside any git repo.
    const { stdout } = await runSkilltap(
      ["status", "--json"],
      homeDir,
      configDir,
      homeDir,
    );
    const report = JSON.parse(stdout) as { scope: string };
    expect(report.scope).toBe("global");
  });

  test("inside a git repo, scope is reported as project", async () => {
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      const { stdout } = await runSkilltap(
        ["status", "--json"],
        homeDir,
        configDir,
        projectRoot,
      );
      const report = JSON.parse(stdout) as { scope: string };
      expect(report.scope).toBe("project");
    } finally {
      await removeTmpDir(projectRoot);
    }
  });
});

describe("status — drift summary", () => {
  test("manifest with extra entry produces a Drift line in the human render", async () => {
    await disableBuiltinTap(configDir);
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);
      // Plant a manifest pointing at a skill that isn't installed → drift.
      await Bun.write(
        join(projectRoot, "skilltap.toml"),
        `[skills]\nphantom = "https://github.com/example/phantom"\n`,
      );
      // Plant an empty v2 state so gatherStatus has a project context.
      await mkdir(join(projectRoot, ".agents"), { recursive: true });
      await Bun.write(
        join(projectRoot, ".agents", "state.json"),
        JSON.stringify(
          { version: 2, skills: [], plugins: [], mcpServers: [] },
          null,
          2,
        ),
      );

      const { exitCode, stdout } = await runSkilltap(
        ["status"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      // Drift summary line in the renderer.
      expect(stdout).toMatch(/Drift|skilltap sync/);
    } finally {
      await removeTmpDir(projectRoot);
    }
  });
});
