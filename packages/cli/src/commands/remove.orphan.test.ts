import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, type TestEnv, createStandaloneSkillRepo, runSkilltap } from "@skilltap/test-utils";
import { loadInstalled } from "@skilltap/core";

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
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
}

// ─── Test 4: Remove succeeds when directory already missing ──────────────────

describe("remove orphan — remove succeeds when directory already missing", () => {
  test("exits 0, record removed from installed.json, output mentions cleanup", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await disableBuiltinTap(configDir);

      // Install the skill
      const install = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(install.exitCode).toBe(0);

      // Manually delete the installed directory (leaving installed.json record)
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Remove should succeed even though directory is already gone
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "remove", "standalone-skill", "--yes", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      // Output should mention the skill was removed
      expect(stdout).toContain("standalone-skill");

      // Record should be removed from installed.json
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});
