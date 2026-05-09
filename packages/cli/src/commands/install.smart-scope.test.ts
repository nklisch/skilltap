/**
 * Subprocess tests for the smart-scope default in `install` (Unit 3.20).
 *
 * Covers the inference rule encoded in `resolveScope`: when no `--scope` flag
 * and no `defaults.scope` config key are set, the cwd's git context decides:
 *   - inside a git repo → `project` (state lands at <root>/.agents/state.json)
 *   - outside any repo  → `global`  (state lands at <SKILLTAP_HOME>)
 *
 * Both branches are exercised without passing `--scope`.
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

describe("install — smart-scope default", () => {
  test("inside a git repo, install (no --scope) lands at <root>/.agents/", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await initRepo(projectRoot);

      const { exitCode, stdout } = await runSkilltap(
        ["install", "skill", repo.path, "--yes", "--skip-scan"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      // Smart-scope reports the inferred choice on stdout.
      expect(stdout.toLowerCase()).toMatch(/scope.*project|project.*inferred/);

      const projectState = await loadState(projectRoot);
      expect(projectState.ok).toBe(true);
      if (!projectState.ok) return;
      expect(
        projectState.value.skills.find((s) => s.name === "standalone-skill"),
      ).toBeDefined();
      expect(
        projectState.value.skills.find((s) => s.name === "standalone-skill")
          ?.scope,
      ).toBe("project");

      // Global state must be empty.
      const globalState = await loadState();
      expect(globalState.ok).toBe(true);
      if (!globalState.ok) return;
      expect(globalState.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("outside any git repo, install (no --scope) lands at global", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createStandaloneSkillRepo();
    // cwd = homeDir (not a git repo) → smart-scope = global.
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["install", "skill", repo.path, "--yes", "--skip-scan"],
        homeDir,
        configDir,
        homeDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout.toLowerCase()).toMatch(/scope.*global|global.*inferred/);

      const globalState = await loadState();
      expect(globalState.ok).toBe(true);
      if (!globalState.ok) return;
      expect(
        globalState.value.skills.find((s) => s.name === "standalone-skill"),
      ).toBeDefined();
      expect(
        globalState.value.skills.find((s) => s.name === "standalone-skill")
          ?.scope,
      ).toBe("global");
    } finally {
      await repo.cleanup();
    }
  });
});
