import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, type TestEnv, commitAll, createStandaloneSkillRepo, initRepo, runSkilltap, makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
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

// ─── Test 3: Agent mode installs through phantom conflict ─────────────────────

describe("install orphan — agent mode installs through phantom conflict", () => {
  test("exits 0 and installs fresh when stale record exists but directory is gone", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await disableBuiltinTap(configDir);

      // First install
      const first = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(first.exitCode).toBe(0);

      // Delete the installed directory (leaving the installed.json record intact)
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Enable agent mode and try to re-install from the same repo
      await mkdir(join(configDir, "skilltap"), { recursive: true });
      await Bun.write(
        join(configDir, "skilltap", "config.toml"),
        `builtin_tap = false\n["agent-mode"]\nenabled = true\nscope = "global"\n`,
      );

      const second = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(second.exitCode).toBe(0);
      expect(second.stdout).toContain("OK:");

      // Exactly one record in installed.json
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Test 8: Install from repo with plugins/*/skills/*/SKILL.md layout ───────

describe("install orphan — plugins layout (plugins/*/skills/*/SKILL.md)", () => {
  test("finds and installs skill in Claude Code plugin convention layout", async () => {
    const repoDir = await makeTmpDir();
    try {
      await disableBuiltinTap(configDir);

      // Create plugins/my-plugin/skills/my-skill/SKILL.md layout
      const skillDir = join(repoDir, "plugins", "my-plugin", "skills", "my-skill");
      await mkdir(skillDir, { recursive: true });
      await Bun.write(
        join(skillDir, "SKILL.md"),
        "---\nname: my-skill\ndescription: Plugin layout skill\n---\n# My Skill\nContent here.\n",
      );
      await initRepo(repoDir);
      await commitAll(repoDir, "initial commit");

      const { exitCode, stdout } = await runSkilltap(
        ["install", repoDir, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("my-skill");

      // Verify it appears in installed.json
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills.some((s) => s.name === "my-skill")).toBe(true);
    } finally {
      await removeTmpDir(repoDir);
    }
  });
});

// ─── Test 11: Re-install skill after manually deleting its directory ──────────

describe("install orphan — re-install after manual directory deletion", () => {
  test("fresh install succeeds with no 'already installed' conflict", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await disableBuiltinTap(configDir);

      // Initial install
      const first = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(first.exitCode).toBe(0);

      // Manually delete the install directory (but leave installed.json record)
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Re-install from the same repo — should succeed, not error about "already installed"
      const second = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(second.exitCode).toBe(0);

      // Exactly one record in installed.json (no duplicates)
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(1);
      expect(installed.value.skills[0]?.name).toBe("standalone-skill");

      // Directory should exist again
      const stat = await Bun.file(join(skillDir, "SKILL.md")).exists();
      expect(stat).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});
