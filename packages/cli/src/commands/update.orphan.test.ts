import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import {
  addFileAndCommit,
  commitAll,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

setDefaultTimeout(60_000);

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

async function disableBuiltinTap(configDir: string): Promise<void> {
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
}

async function writeAgentModeConfig(configDir: string): Promise<void> {
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(
    join(configDir, "skilltap", "config.toml"),
    `builtin_tap = false\n["agent-mode"]\nenabled = true\nscope = "global"\n`,
  );
}

// ─── Test 1: Agent mode auto-cleans orphan record during update ───────────────

describe("update orphan — agent mode auto-cleans orphan during update", () => {
  test("exits 0 and reports stale record with Auto-removing", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await disableBuiltinTap(configDir);
      const install = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(install.exitCode).toBe(0);

      // Delete the installed skill directory to create an orphan
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Enable agent mode
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Stale record");
      expect(stdout).toContain("Auto-removing");
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Test 2: agent mode auto-cleans orphan and healthy skills still update ────

describe("update orphan — agent mode cleans orphan, healthy skill still updates", () => {
  test("exits 0 and cleans stale record while processing healthy skills", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await disableBuiltinTap(configDir);
      const install = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(install.exitCode).toBe(0);

      // Delete the installed skill directory to create an orphan
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Enable agent mode for auto-cleanup
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      // Should warn about stale record and auto-clean
      expect(stdout).toMatch(/[Ss]tale|Auto-removing/);
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Test 7: Update with one orphan and one healthy skill ────────────────────

describe("update orphan — one orphan + one healthy skill", () => {
  test("cleans orphan and updates healthy skill in one pass", async () => {
    const repoA = await createStandaloneSkillRepo();
    const repoB = await makeTmpDir();
    let repoBCleanup: (() => Promise<void>) | null = null;

    try {
      await disableBuiltinTap(configDir);

      // Create second standalone skill repo
      await mkdir(repoB, { recursive: true });
      await Bun.write(
        join(repoB, "SKILL.md"),
        "---\nname: second-skill\ndescription: Second test skill\n---\n# Second Skill\n",
      );
      await initRepo(repoB);
      await commitAll(repoB, "initial commit");
      repoBCleanup = () => removeTmpDir(repoB);

      // Install both skills (before agent mode so --skip-scan works)
      await runSkilltap(
        ["install", repoA.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      await runSkilltap(
        ["install", repoB, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );

      // Delete one skill directory (make it an orphan)
      const orphanDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(orphanDir, { recursive: true, force: true });

      // Add a new commit to the healthy skill's repo
      await addFileAndCommit(repoB, "extra.md", "# Extra\nNew content.", "update");

      // Enable agent mode so orphan cleanup is automatic
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      // Orphan cleaned
      expect(stdout).toMatch(/[Ss]tale|Auto-removing/);
      // Healthy skill updated
      expect(stdout).toContain("OK: Updated second-skill");
    } finally {
      await repoA.cleanup();
      if (repoBCleanup) await repoBCleanup();
    }
  });
});

// ─── Test 9: Multi-skill cache subdirectory removed upstream (THE crash fix) ──

describe("update orphan — multi-skill cache subdir removed upstream (crash fix)", () => {
  test("does NOT crash when upstream removes a skill subdirectory", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await disableBuiltinTap(configDir);

      // Install both skills from multi-skill repo
      const install = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(install.exitCode).toBe(0);

      // In the SOURCE repo, remove skill-b's subdirectory and commit
      await rm(join(repo.path, ".agents", "skills", "skill-b"), { recursive: true, force: true });
      await commitAll(repo.path, "remove skill-b");

      // Run update — must NOT crash even though skill-b's cache subdir will be gone after pull
      const { exitCode, stderr } = await runSkilltap(
        ["update", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      // No cp error in stderr
      expect(stderr).not.toContain("cannot stat");
      expect(stderr).not.toContain("cp:");
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Test 10: All installed skills are orphaned ───────────────────────────────

describe("update orphan — all installed skills are orphaned", () => {
  test("exits 0 and cleans both stale records when all skills deleted", async () => {
    const repoA = await createStandaloneSkillRepo();
    const repoB = await makeTmpDir();
    let repoBCleanup: (() => Promise<void>) | null = null;

    try {
      await disableBuiltinTap(configDir);

      // Create second standalone skill repo
      await Bun.write(
        join(repoB, "SKILL.md"),
        "---\nname: second-skill\ndescription: Second test skill\n---\n# Second Skill\n",
      );
      await initRepo(repoB);
      await commitAll(repoB, "initial commit");
      repoBCleanup = () => removeTmpDir(repoB);

      // Install both skills (before agent mode so --skip-scan works)
      await runSkilltap(
        ["install", repoA.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      await runSkilltap(
        ["install", repoB, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );

      // Delete BOTH skill directories
      await rm(join(homeDir, ".agents", "skills", "standalone-skill"), { recursive: true, force: true });
      await rm(join(homeDir, ".agents", "skills", "second-skill"), { recursive: true, force: true });

      // Enable agent mode for auto-cleanup
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      // Both stale records should be reported/cleaned
      expect(stdout).toMatch(/[Ss]tale|Auto-removing/);
      // Should mention both skills
      expect(stdout).toContain("standalone-skill");
      expect(stdout).toContain("second-skill");
    } finally {
      await repoA.cleanup();
      if (repoBCleanup) await repoBCleanup();
    }
  });
});

// ─── Test 12: Orphan cleanup is idempotent — running update twice ─────────────

describe("update orphan — idempotent cleanup (run twice)", () => {
  test("first run cleans, second run is clean — both exit 0", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      // Install before enabling agent mode (so --skip-scan works)
      await disableBuiltinTap(configDir);
      await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );

      // Delete the installed skill directory to create an orphan
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(skillDir, { recursive: true, force: true });

      // Enable agent mode for auto-cleanup
      await writeAgentModeConfig(configDir);

      // First run: warns and cleans
      const first = await runSkilltap(["update"], homeDir, configDir);
      expect(first.exitCode).toBe(0);
      expect(first.stdout).toMatch(/[Ss]tale|Auto-removing/);

      // Second run: clean pass, no more orphans, exits 0
      const second = await runSkilltap(["update"], homeDir, configDir);
      expect(second.exitCode).toBe(0);
      expect(second.stdout).not.toMatch(/[Ss]tale|Auto-removing/);
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Test 13: Mixed orphan types cleaned in one update ───────────────────────

describe("update orphan — mixed orphan types cleaned together", () => {
  test("cleans directory-missing and link-target-missing orphans in one pass", async () => {
    const repoA = await createStandaloneSkillRepo();
    const repoB = await makeTmpDir();
    let repoBCleanup: (() => Promise<void>) | null = null;

    try {
      await disableBuiltinTap(configDir);

      // Create a skill directory to link (not a git repo, just a directory with SKILL.md)
      await Bun.write(
        join(repoB, "SKILL.md"),
        "---\nname: linked-skill\ndescription: Linked test skill\n---\n# Linked Skill\n",
      );
      repoBCleanup = () => removeTmpDir(repoB);

      // Install first skill (will become directory-missing orphan)
      await runSkilltap(
        ["install", repoA.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );

      // Link second skill directory (will become link-target-missing orphan)
      await runSkilltap(["link", repoB, "--global"], homeDir, configDir);

      // Create directory-missing orphan: delete install dir
      const installDir = join(homeDir, ".agents", "skills", "standalone-skill");
      await rm(installDir, { recursive: true, force: true });

      // Create link-target-missing orphan: delete the linked directory
      await rm(repoB, { recursive: true, force: true });
      repoBCleanup = null; // already deleted

      // Enable agent mode for auto-cleanup
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      // Both orphan types should be cleaned
      expect(stdout).toMatch(/[Ss]tale|Auto-removing/);
    } finally {
      await repoA.cleanup();
      if (repoBCleanup) await repoBCleanup();
    }
  });
});

// ─── Test 14: Agent mode with stale multi-skill record (cache completely gone) ─

describe("update orphan — agent mode, multi-skill cache completely deleted", () => {
  test("warns about stale records and exits 0 when cache dir deleted", async () => {
    const repo = await createMultiSkillRepo();
    try {
      // Install before enabling agent mode (so --skip-scan works)
      await disableBuiltinTap(configDir);
      const install = await runSkilltap(
        ["install", repo.path, "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(install.exitCode).toBe(0);

      // Delete the entire git cache directory for this repo
      // Cache lives at XDG_CONFIG_HOME/skilltap/cache/
      const cacheBaseDir = join(configDir, "skilltap", "cache");
      await rm(cacheBaseDir, { recursive: true, force: true });

      // Enable agent mode for auto-cleanup
      await writeAgentModeConfig(configDir);

      const { exitCode, stdout } = await runSkilltap(["update"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toMatch(/[Ss]tale|Auto-removing/);
    } finally {
      await repo.cleanup();
    }
  });
});
