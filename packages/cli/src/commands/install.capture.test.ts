/**
 * CLI subprocess tests for plugin capture (Unit 4 of Phase 39).
 *
 * Tests exercise the capture callback wiring via runSkilltap (pipe mode).
 * PTY-dependent interactive-prompt tests are omitted per testing.md §Test
 * selection (runSkilltap runs in pipe mode; clack prompts don't render fully).
 *
 * --yes is used to bypass interactive prompts; cross-source conflicts abort
 * automatically in pipe mode (clack prompts return cancel symbol → abort).
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
import { loadSkillState, loadPlugins } from "@skilltap/core";
import {
  commitAll,
  createClaudePluginRepo,
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

// ---------------------------------------------------------------------------
// Config helpers
// ---------------------------------------------------------------------------

async function disableBuiltinTap(configDir: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), "builtin_tap = false\n");
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/**
 * Create a minimal git repo containing only a `helper/SKILL.md`.
 * Used as a "different source" for cross-source conflict tests.
 */
async function createHelperSkillRepo(): Promise<{
  path: string;
  cleanup: () => Promise<void>;
}> {
  const dir = await makeTmpDir();
  await mkdir(join(dir, "helper"), { recursive: true });
  await Bun.write(
    join(dir, "helper", "SKILL.md"),
    `---\nname: helper\ndescription: A helper skill from a different source\n---\n# Helper\nTest.\n`,
  );
  await initRepo(dir);
  await commitAll(dir);
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

/**
 * Seed the helper standalone from a given repo path via the CLI
 * using interactive --yes mode (no plugin detection callback → skills-only
 * path when plugin manifest is absent; fails if plugin manifest is present
 * and auto-selects "plugin" in --yes mode).
 *
 * For repos that contain a plugin manifest (like claude-plugin), we use a
 * Bun subprocess with the core API to seed skills-only.
 */
async function seedHelperFromCoreApi(
  repoPath: string,
  homeDir: string,
  configDir: string,
  onPluginDetected: "plugin" | "skills-only" = "skills-only",
): Promise<boolean> {
  // Write a small inline script that calls installSkill with the desired
  // onPluginDetected callback, then run it in the test env.
  const coreEntry = join(
    import.meta.dir,
    "../../../core/src/index.ts",
  );
  const script = `
import { installSkill } from ${JSON.stringify(coreEntry)};
const result = await installSkill(${JSON.stringify(repoPath)}, {
  scope: "global",
  skipScan: true,
  onPluginDetected: async () => ${JSON.stringify(onPluginDetected)},
});
if (!result.ok) {
  process.stderr.write("seed failed: " + result.error.message + "\\n");
  process.exit(1);
}
process.stdout.write("ok\\n");
`.trim();

  const scriptPath = join(homeDir, "_seed.ts");
  await Bun.write(scriptPath, script);

  const proc = Bun.spawn(["bun", "run", "--bun", scriptPath], {
    cwd: homeDir,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      SKILLTAP_HOME: homeDir,
      XDG_CONFIG_HOME: configDir,
    },
  });
  const exitCode = await proc.exited;
  return exitCode === 0;
}

// ---------------------------------------------------------------------------
// No matching standalones → normal plugin install
// ---------------------------------------------------------------------------

describe("install capture — no overlap", () => {
  test("plugin install with no pre-existing standalones completes normally", async () => {
    await disableBuiltinTap(configDir);
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["install", "plugin", pluginRepo.path, "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("test-plugin");
    } finally {
      await pluginRepo.cleanup();
    }
  });

  test("no capture summary line when no standalones were captured", async () => {
    await disableBuiltinTap(configDir);
    const pluginRepo = await createClaudePluginRepo();
    try {
      const { stdout } = await runSkilltap(
        ["install", "plugin", pluginRepo.path, "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(stdout).not.toContain("Captured");
    } finally {
      await pluginRepo.cleanup();
    }
  });
});

// ---------------------------------------------------------------------------
// Cross-source conflict → hard-abort (clack prompt cancels in pipe mode)
// ---------------------------------------------------------------------------

describe("install capture — cross-source conflict", () => {
  test("exits non-zero when standalone installed from different source", async () => {
    await disableBuiltinTap(configDir);
    const helperRepo = await createHelperSkillRepo();
    const pluginRepo = await createClaudePluginRepo();
    try {
      // Seed: install helper standalone from helperRepo
      const seeded = await seedHelperFromCoreApi(helperRepo.path, homeDir, configDir, "skills-only");
      expect(seeded).toBe(true);

      // Verify standalone is recorded
      const before = await loadSkillState();
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      expect(before.value.skills.some((s) => s.name === "helper")).toBe(true);

      // Install plugin from pluginRepo (different canonical source → cross-source)
      // In pipe mode, the cross-source conflict prompt is cancelled (non-TTY)
      // which causes the install to abort → exit non-zero.
      const { exitCode, stdout, stderr } = await runSkilltap(
        ["install", "plugin", pluginRepo.path, "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);

      // Error output should mention conflict / substitution / different source
      const combined = stdout + stderr;
      expect(combined).toMatch(
        /different source|cross.source|conflict|replace|standalone/i,
      );
    } finally {
      await helperRepo.cleanup();
      await pluginRepo.cleanup();
    }
  });

  test("standalone state unchanged after cross-source conflict abort", async () => {
    await disableBuiltinTap(configDir);
    const helperRepo = await createHelperSkillRepo();
    const pluginRepo = await createClaudePluginRepo();
    try {
      const seeded = await seedHelperFromCoreApi(helperRepo.path, homeDir, configDir, "skills-only");
      expect(seeded).toBe(true);

      await runSkilltap(
        ["install", "plugin", pluginRepo.path, "--yes", "--global"],
        homeDir,
        configDir,
      );

      // Standalone still present
      const after = await loadSkillState();
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      expect(after.value.skills.some((s) => s.name === "helper")).toBe(true);

      // Plugin NOT recorded
      const plugins = await loadPlugins();
      expect(plugins.ok).toBe(true);
      if (!plugins.ok) return;
      expect(plugins.value.plugins.some((p) => p.name === "test-plugin")).toBe(
        false,
      );
    } finally {
      await helperRepo.cleanup();
      await pluginRepo.cleanup();
    }
  });
});

// ---------------------------------------------------------------------------
// Same-source capture → auto-confirm with --yes, summary in output
// ---------------------------------------------------------------------------

describe("install capture — same-source capture", () => {
  test("captures same-source standalone, emits Captured summary, exits 0", async () => {
    await disableBuiltinTap(configDir);
    const pluginRepo = await createClaudePluginRepo();
    try {
      // Seed: install helper as skills-only from the same plugin repo path.
      // Same local path → same canonical source → sameSource match.
      const seeded = await seedHelperFromCoreApi(pluginRepo.path, homeDir, configDir, "skills-only");
      expect(seeded).toBe(true);

      // Verify standalone seeded
      const before = await loadSkillState();
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      expect(before.value.skills.some((s) => s.name === "helper")).toBe(true);

      // Install same repo as plugin → same-source capture → auto-confirm with --yes
      const { exitCode, stdout } = await runSkilltap(
        ["install", "plugin", pluginRepo.path, "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      // Capture summary should be in output
      expect(stdout).toContain("Captured");
      expect(stdout).toContain("test-plugin");

      // Standalone no longer in state
      const after = await loadSkillState();
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      expect(after.value.skills.some((s) => s.name === "helper")).toBe(false);

      // Plugin recorded
      const plugins = await loadPlugins();
      expect(plugins.ok).toBe(true);
      if (!plugins.ok) return;
      expect(
        plugins.value.plugins.some((p) => p.name === "test-plugin"),
      ).toBe(true);
    } finally {
      await pluginRepo.cleanup();
    }
  });
});
