import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat, mkdir, symlink } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, makeTmpDir, removeTmpDir, type TestEnv } from "@skilltap/test-utils";
import { loadInstalled, saveInstalled } from "./config";
import { skillCacheDir, skillInstallDir } from "./paths";
import {
  findOrphanRecords,
  formatOrphanReason,
  purgeOrphanRecords,
} from "./orphan";
import type { InstalledJson, InstalledSkill } from "./schemas/installed";
import { removeSkill } from "./remove";

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

const NOW = "2024-01-01T00:00:00.000Z";

function makeSkill(overrides: Partial<InstalledSkill>): InstalledSkill {
  return {
    name: "test-skill",
    description: "A test skill",
    repo: "https://github.com/example/repo",
    ref: null,
    sha: null,
    scope: "global",
    path: null,
    tap: null,
    also: [],
    installedAt: NOW,
    updatedAt: NOW,
    active: true,
    ...overrides,
  };
}

function makeInstalled(skills: InstalledSkill[]): InstalledJson {
  return { version: 1, skills };
}

describe("formatOrphanReason", () => {
  test("formats directory-missing", () => {
    expect(formatOrphanReason("directory-missing")).toBe(
      "install directory missing from disk",
    );
  });

  test("formats cache-missing", () => {
    expect(formatOrphanReason("cache-missing")).toBe(
      "git cache directory missing",
    );
  });

  test("formats cache-subdir-missing", () => {
    expect(formatOrphanReason("cache-subdir-missing")).toBe(
      "skill subdirectory removed from upstream repo",
    );
  });

  test("formats link-target-missing", () => {
    expect(formatOrphanReason("link-target-missing")).toBe(
      "symlink target no longer exists",
    );
  });
});

describe("findOrphanRecords", () => {
  test("returns empty for skill with existing directory", async () => {
    const skill = makeSkill({ name: "healthy-skill" });
    const installDir = skillInstallDir("healthy-skill", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects directory-missing for standalone skill", async () => {
    const skill = makeSkill({ name: "missing-skill" });
    // Do NOT create the directory
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.record.name).toBe("missing-skill");
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("skips disabled skills entirely", async () => {
    const skill = makeSkill({ name: "disabled-skill", active: false });
    // Do NOT create any directory — disabled skills should never be flagged
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects cache-missing for multi-skill", async () => {
    const skill = makeSkill({
      name: "multi-skill",
      path: ".agents/skills/multi-skill",
    });
    // Do NOT create the cache dir
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("cache-missing");
  });

  test("detects cache-subdir-missing when cache exists but subdirectory doesn't", async () => {
    const repoUrl = "https://github.com/example/multi-repo";
    const subdir = ".agents/skills/multi-skill";
    const skill = makeSkill({
      name: "multi-skill",
      repo: repoUrl,
      path: subdir,
    });

    // Create cache dir with .git but NOT the subdir
    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    // Also create install dir so that's not the problem
    const installDir = skillInstallDir("multi-skill", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("cache-subdir-missing");
  });

  test("no orphan when cache and subdir both exist", async () => {
    const repoUrl = "https://github.com/example/multi-repo2";
    const subdir = ".agents/skills/healthy-multi";
    const skill = makeSkill({
      name: "healthy-multi",
      repo: repoUrl,
      path: subdir,
    });

    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    await mkdir(join(cacheDir, subdir), { recursive: true });
    const installDir = skillInstallDir("healthy-multi", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects link-target-missing for linked skill", async () => {
    const skill = makeSkill({
      name: "linked-skill",
      scope: "linked",
      path: join(homeDir, "nonexistent-target"),
    });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("link-target-missing");
  });

  test("no orphan for linked skill whose target exists", async () => {
    const targetDir = join(homeDir, "existing-linked-skill");
    await mkdir(targetDir, { recursive: true });

    const skill = makeSkill({
      name: "linked-skill",
      scope: "linked",
      path: targetDir,
    });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects directory-missing for npm skill", async () => {
    const skill = makeSkill({
      name: "npm-skill",
      repo: "npm:@example/npm-skill",
    });
    // Do NOT create install dir
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("handles mixed orphans and healthy records", async () => {
    const healthySkill = makeSkill({ name: "healthy" });
    const installDir = skillInstallDir("healthy", "global");
    await mkdir(installDir, { recursive: true });

    const orphanSkill = makeSkill({ name: "orphan" });
    // No dir created for orphan

    const installed = makeInstalled([healthySkill, orphanSkill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.record.name).toBe("orphan");
  });
});

describe("purgeOrphanRecords", () => {
  test("removes specified orphan records from installed.json", async () => {
    const skill = makeSkill({ name: "orphan-skill" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const orphans = [{ record: skill, reason: "directory-missing" as const }];
    const result = await purgeOrphanRecords(orphans, installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("orphan-skill");
    expect(installed.skills).toHaveLength(0);
  });

  test("saves installed.json after purging", async () => {
    const skill = makeSkill({ name: "to-purge" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const orphans = [{ record: skill, reason: "directory-missing" as const }];
    await purgeOrphanRecords(orphans, installed);

    // Reload from disk and verify
    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(0);
  });

  test("returns names of purged records", async () => {
    const skill1 = makeSkill({ name: "skill-a" });
    const skill2 = makeSkill({ name: "skill-b" });
    const installed = makeInstalled([skill1, skill2]);
    await saveInstalled(installed);

    const orphans = [
      { record: skill1, reason: "directory-missing" as const },
      { record: skill2, reason: "cache-missing" as const },
    ];
    const result = await purgeOrphanRecords(orphans, installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("skill-a");
    expect(result.value).toContain("skill-b");
  });

  test("handles empty orphan list (no-op)", async () => {
    const skill = makeSkill({ name: "healthy" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const result = await purgeOrphanRecords([], installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
    expect(installed.skills).toHaveLength(1);
  });

  test("preserves healthy records when purging orphans", async () => {
    const healthySkill = makeSkill({ name: "healthy" });
    const orphanSkill = makeSkill({ name: "orphan" });
    const installed = makeInstalled([healthySkill, orphanSkill]);
    await saveInstalled(installed);

    const orphans = [{ record: orphanSkill, reason: "directory-missing" as const }];
    await purgeOrphanRecords(orphans, installed);

    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(1);
    expect(reloaded.value.skills[0]!.name).toBe("healthy");
  });

  // Gap #8: purgeOrphanRecords must remove agent symlinks for purged records
  test("removes agent symlinks for purged records", async () => {
    const skillName = "linked-agent-skill";
    const skill = makeSkill({ name: skillName, also: ["claude-code"] });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    // Create a real target dir and a symlink at the agent path, mimicking what installSkill does.
    // For claude-code at global scope: ${SKILLTAP_HOME}/.claude/skills/<name>
    const agentSkillsDir = join(homeDir, ".claude", "skills");
    const symlinkPath = join(agentSkillsDir, skillName);
    const targetDir = join(homeDir, ".agents", "skills", skillName);
    await mkdir(agentSkillsDir, { recursive: true });
    await mkdir(targetDir, { recursive: true });
    await symlink(targetDir, symlinkPath, "dir");

    // Verify the symlink exists before purge
    const beforeStat = await lstat(symlinkPath).catch(() => null);
    expect(beforeStat).not.toBeNull();
    expect(beforeStat!.isSymbolicLink()).toBe(true);

    const orphans = [{ record: skill, reason: "directory-missing" as const }];
    const result = await purgeOrphanRecords(orphans, installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // The agent symlink should be removed
    const afterStat = await lstat(symlinkPath).catch(() => null);
    expect(afterStat).toBeNull();
  });
});

// ─── Gap tests: findOrphanRecords edge cases ───────────────────────────────

describe("findOrphanRecords — additional cases", () => {
  // Gap #22: project-scoped skill orphan detection
  test("detects directory-missing for project-scoped skill", async () => {
    const projectRoot = await makeTmpDir();
    try {
      const skill = makeSkill({ name: "project-skill", scope: "project" });
      // Do NOT create the project-scoped install dir
      const installed = makeInstalled([skill]);

      const orphans = await findOrphanRecords(installed, projectRoot);
      expect(orphans).toHaveLength(1);
      expect(orphans[0]!.record.name).toBe("project-skill");
      expect(orphans[0]!.reason).toBe("directory-missing");
    } finally {
      await removeTmpDir(projectRoot);
    }
  });

  test("no orphan when project-scoped skill directory exists", async () => {
    const projectRoot = await makeTmpDir();
    try {
      const skill = makeSkill({ name: "project-skill", scope: "project" });
      const installDir = join(projectRoot, ".agents", "skills", "project-skill");
      await mkdir(installDir, { recursive: true });
      const installed = makeInstalled([skill]);

      const orphans = await findOrphanRecords(installed, projectRoot);
      expect(orphans).toHaveLength(0);
    } finally {
      await removeTmpDir(projectRoot);
    }
  });

  // Gap #24: local skill (repo === null) directory missing
  test("detects directory-missing for local skill (repo null)", async () => {
    const skill = makeSkill({ name: "local-skill", repo: null });
    // Do NOT create the install directory
    const installed = makeInstalled([skill]);

    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.record.name).toBe("local-skill");
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("no orphan for local skill (repo null) when directory exists", async () => {
    const skill = makeSkill({ name: "local-skill", repo: null });
    const installDir = skillInstallDir("local-skill", "global");
    await mkdir(installDir, { recursive: true });
    const installed = makeInstalled([skill]);

    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  // Gap #21: multi-skill with cache-subdir-missing takes priority over directory-missing
  test("multi-skill reports cache-subdir-missing (not directory-missing) when cache exists but subdir is gone", async () => {
    // The implementation checks cache first, then subdir, then install dir.
    // When cache exists but subdir is missing, it reports cache-subdir-missing
    // and continues (skips the install dir check). So only ONE orphan per record.
    const repoUrl = "https://github.com/example/priority-test-repo";
    const subdir = ".agents/skills/priority-skill";
    const skill = makeSkill({
      name: "priority-skill",
      repo: repoUrl,
      path: subdir,
    });

    // Cache exists with .git, but subdir is missing AND install dir is also missing
    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    // Do NOT create the subdir in cache
    // Do NOT create the install dir

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);

    // Should report exactly one orphan — the most specific: cache-subdir-missing
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("cache-subdir-missing");
  });

  test("multi-skill reports directory-missing when cache and subdir exist but install dir is gone", async () => {
    const repoUrl = "https://github.com/example/installdir-missing-repo";
    const subdir = ".agents/skills/installdir-skill";
    const skill = makeSkill({
      name: "installdir-skill",
      repo: repoUrl,
      path: subdir,
    });

    // Cache and subdir exist, but install dir is missing
    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    await mkdir(join(cacheDir, subdir), { recursive: true });
    // Do NOT create install dir

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);

    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("directory-missing");
  });
});

// ─── Gap #20: removeSkill calls onOrphanRemoved callback ──────────────────

describe("removeSkill — orphan callback", () => {
  test("calls onOrphanRemoved when directory is already missing", async () => {
    const skill = makeSkill({ name: "ghost-skill" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);
    // Do NOT create the install directory

    const orphanRemovedNames: string[] = [];
    const result = await removeSkill("ghost-skill", {
      onOrphanRemoved(name) {
        orphanRemovedNames.push(name);
      },
    });

    expect(result.ok).toBe(true);
    expect(orphanRemovedNames).toContain("ghost-skill");
  });

  test("does not call onOrphanRemoved when directory exists", async () => {
    const skill = makeSkill({ name: "real-skill" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    // Create the install directory
    const installDir = skillInstallDir("real-skill", "global");
    await mkdir(installDir, { recursive: true });

    const orphanRemovedNames: string[] = [];
    const result = await removeSkill("real-skill", {
      onOrphanRemoved(name) {
        orphanRemovedNames.push(name);
      },
    });

    expect(result.ok).toBe(true);
    expect(orphanRemovedNames).toHaveLength(0);
  });
});
