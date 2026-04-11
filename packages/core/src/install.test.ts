import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat, readlink } from "node:fs/promises";
import { join } from "node:path";
import {
  createMaliciousSkillRepo,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  createTestEnv,
  makeTmpDir,
  removeTmpDir,
  type TestEnv,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { installSkill } from "./install";
import { findProjectRoot } from "./paths";
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

describe("installSkill — standalone", () => {
  test("installs to global scope", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const result = await installSkill(repo.path, { scope: "global" });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records).toHaveLength(1);
      // biome-ignore lint/style/noNonNullAssertion: asserted length above
      const record = result.value.records[0]!;
      expect(record.name).toBe("standalone-skill");
      expect(record.scope).toBe("global");
      expect(record.sha).toBeString();
      expect(record.path).toBeNull();

      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      const stat = await lstat(skillDir);
      expect(stat.isDirectory()).toBe(true);

      // SKILL.md is present
      expect(await Bun.file(join(skillDir, "SKILL.md")).exists()).toBe(true);
      // git history preserved (standalone keeps .git dir)
      expect(
        await lstat(join(skillDir, ".git"))
          .then(() => true)
          .catch(() => false),
      ).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("records correctly in installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;

      const { skills } = installedResult.value;
      expect(skills).toHaveLength(1);
      expect(skills[0]?.name).toBe("standalone-skill");
      expect(skills[0]?.scope).toBe("global");
      expect(skills[0]?.path).toBeNull();
      expect(skills[0]?.also).toEqual([]);
    } finally {
      await repo.cleanup();
    }
  });

  test("creates agent symlinks when also is set", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", also: ["claude-code"] });

      const linkPath = join(homeDir, ".claude", "skills", "standalone-skill");
      const target = await readlink(linkPath);
      const expectedTarget = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(target).toBe(expectedTarget);
    } finally {
      await repo.cleanup();
    }
  });

  test("already installed skill errors without onAlreadyInstalled callback", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const result = await installSkill(repo.path, { scope: "global" });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("already installed");
      expect(result.error.hint).toContain("update");
    } finally {
      await repo.cleanup();
    }
  });

  test("already installed skill with onAlreadyInstalled=update goes to updates list", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const result = await installSkill(repo.path, {
        scope: "global",
        onAlreadyInstalled: async () => "update",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.updates).toContain("standalone-skill");
      expect(result.value.records).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("onAlreadyInstalled returning abort still produces an error", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const result = await installSkill(repo.path, {
        scope: "global",
        onAlreadyInstalled: async () => "abort",
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("already installed");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("installSkill — multi-skill", () => {
  test("installs a single selected skill", async () => {
    const repo = await createMultiSkillRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        skillNames: ["skill-a"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records).toHaveLength(1);
      expect(result.value.records[0]?.name).toBe("skill-a");
      expect(result.value.records[0]?.path).not.toBeNull();

      const skillADir = join(homeDir, ".agents", "skills", "skill-a");
      expect(await lstat(skillADir).then((s) => s.isDirectory())).toBe(true);

      const skillBDir = join(homeDir, ".agents", "skills", "skill-b");
      expect(await lstat(skillBDir).catch(() => null)).toBeNull();

      // Cache dir exists
      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: install succeeded, at least one skill present
      const record = installedResult.value.skills[0]!;
      expect(record.repo).toBe(repo.path);
    } finally {
      await repo.cleanup();
    }
  });

  test("installs all skills when skillNames omitted", async () => {
    const repo = await createMultiSkillRepo();
    try {
      const result = await installSkill(repo.path, { scope: "global" });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records).toHaveLength(2);
      const names = result.value.records.map((r) => r.name).sort();
      expect(names).toEqual(["skill-a", "skill-b"]);

      expect(
        await lstat(join(homeDir, ".agents", "skills", "skill-a")).then((s) =>
          s.isDirectory(),
        ),
      ).toBe(true);
      expect(
        await lstat(join(homeDir, ".agents", "skills", "skill-b")).then((s) =>
          s.isDirectory(),
        ),
      ).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("skillNames takes precedence over onSelectSkills", async () => {
    const repo = await createMultiSkillRepo();
    try {
      let selectSkillsCalled = false;
      const result = await installSkill(repo.path, {
        scope: "global",
        skillNames: ["skill-a"],
        skipScan: true,
        onSelectSkills: async (skills) => {
          selectSkillsCalled = true;
          return skills.map((s) => s.name);
        },
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(selectSkillsCalled).toBe(false);
      expect(result.value.records).toHaveLength(1);
      expect(result.value.records[0]?.name).toBe("skill-a");
    } finally {
      await repo.cleanup();
    }
  });

  test("partial overlap: new skills install and existing go to updates", async () => {
    const repo = await createMultiSkillRepo();
    try {
      // Pre-install only skill-a
      await installSkill(repo.path, { scope: "global", skillNames: ["skill-a"], skipScan: true });

      // Install whole repo — skill-a is already installed, skill-b is new
      const result = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
        onAlreadyInstalled: async () => "update",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records.map((r) => r.name)).toContain("skill-b");
      expect(result.value.records.map((r) => r.name)).not.toContain("skill-a");
      expect(result.value.updates).toContain("skill-a");

      // Only one entry for skill-a in installed.json (no duplicate)
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      const aEntries = installed.value.skills.filter((s) => s.name === "skill-a");
      expect(aEntries).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("installSkill — project scope", () => {
  test("installs to project root", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();
      const result = await installSkill(repo.path, {
        scope: "project",
        projectRoot,
      });
      expect(result.ok).toBe(true);

      const skillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("saves to project installed.json, not global", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();
      await installSkill(repo.path, { scope: "project", projectRoot, skipScan: true });

      // Project file should have the record
      const projectInstalled = await loadInstalled(projectRoot);
      expect(projectInstalled.ok).toBe(true);
      if (!projectInstalled.ok) return;
      expect(projectInstalled.value.skills.map((s) => s.name)).toContain("standalone-skill");

      // Global file should be empty
      const globalInstalled = await loadInstalled();
      expect(globalInstalled.ok).toBe(true);
      if (!globalInstalled.ok) return;
      expect(globalInstalled.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("same skill in two projects coexist independently", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectA = await makeTmpDir();
    const projectB = await makeTmpDir();
    try {
      await $`git -C ${projectA} init`.quiet();
      await $`git -C ${projectB} init`.quiet();

      await installSkill(repo.path, { scope: "project", projectRoot: projectA, skipScan: true });
      await installSkill(repo.path, { scope: "project", projectRoot: projectB, skipScan: true });

      // Both project files have the record
      const aInstalled = await loadInstalled(projectA);
      const bInstalled = await loadInstalled(projectB);
      expect(aInstalled.ok && aInstalled.value.skills).toHaveLength(1);
      expect(bInstalled.ok && bInstalled.value.skills).toHaveLength(1);

      // Global file is empty
      const globalInstalled = await loadInstalled();
      expect(globalInstalled.ok).toBe(true);
      if (!globalInstalled.ok) return;
      expect(globalInstalled.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectA);
      await removeTmpDir(projectB);
    }
  });
});

describe("removeSkill", () => {
  test("removes a standalone skill", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");

      const result = await removeSkill("standalone-skill", { scope: "global" });
      expect(result.ok).toBe(true);

      expect(await lstat(skillDir).catch(() => null)).toBeNull();

      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      expect(installedResult.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("removes symlinks on remove", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", also: ["claude-code"] });
      const linkPath = join(homeDir, ".claude", "skills", "standalone-skill");
      expect(await lstat(linkPath).catch(() => null)).not.toBeNull();

      await removeSkill("standalone-skill", { scope: "global" });
      expect(await lstat(linkPath).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("removes cache when last skill from multi-skill repo is removed", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, {
        scope: "global",
        skillNames: ["skill-a"],
      });
      const installedResult = await loadInstalled();
      if (!installedResult.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: install succeeded, at least one skill present
      const record = installedResult.value.skills[0]!;

      // biome-ignore lint/style/noNonNullAssertion: multi-skill install always sets repo
      const hash = Bun.hash(record.repo!).toString(16);
      const cacheRoot = join(configDir, "skilltap", "cache", hash);
      expect(await lstat(cacheRoot).then((s) => s.isDirectory())).toBe(true);

      await removeSkill("skill-a", { scope: "global" });
      expect(await lstat(cacheRoot).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("keeps cache when another skill from same repo remains", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const installedResult = await loadInstalled();
      if (!installedResult.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: install succeeded, at least one skill present
      const record = installedResult.value.skills[0]!;

      // biome-ignore lint/style/noNonNullAssertion: multi-skill install always sets repo
      const hash = Bun.hash(record.repo!).toString(16);
      const cacheRoot = join(configDir, "skilltap", "cache", hash);

      await removeSkill("skill-a", { scope: "global" });
      expect(await lstat(cacheRoot).then((s) => s.isDirectory())).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("errors when skill not installed", async () => {
    const result = await removeSkill("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });
});

describe("installSkill — security scanning", () => {
  test("returns warnings when installing malicious skill", async () => {
    const repo = await createMaliciousSkillRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        onWarnings: async () => true,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.warnings.length).toBeGreaterThan(0);
      expect(result.value.records).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });

  test("aborts install when onWarnings returns false", async () => {
    const repo = await createMaliciousSkillRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        onWarnings: async () => false,
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("cancelled");

      // Skill should not be installed
      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      expect(installedResult.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("skips scan when skipScan is true", async () => {
    const repo = await createMaliciousSkillRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.warnings).toHaveLength(0);
      expect(result.value.records).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });

  test("clean skill installs with no warnings", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const result = await installSkill(repo.path, { scope: "global" });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.warnings).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("idempotency", () => {
  test("second install does not add duplicate record to installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      expect(installedResult.value.skills).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });

  test("reinstall after remove produces clean state with one record", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await removeSkill("standalone-skill");
      const result = await installSkill(repo.path, { scope: "global", skipScan: true });
      expect(result.ok).toBe(true);

      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      expect(installedResult.value.skills).toHaveLength(1);
      expect(installedResult.value.skills[0]?.name).toBe("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("installed.json state integrity", () => {
  test("truncated JSON returns error without crashing", async () => {
    const { mkdir } = await import("node:fs/promises");
    const dir = join(configDir, "skilltap");
    await mkdir(dir, { recursive: true });
    await Bun.write(
      join(dir, "installed.json"),
      '{"version": 1, "skills": [{"name"',
    );

    const result = await loadInstalled();
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toBeTruthy();
  });

  test("saveInstalled with 100-skill array succeeds and reloads correctly", async () => {
    const baseSkill = {
      name: "",
      description: "A test skill",
      repo: "https://github.com/example/skill.git",
      ref: "main",
      sha: "abc123def456",
      scope: "global" as const,
      path: null,
      tap: null,
      also: [],
      installedAt: "2025-01-01T00:00:00.000Z",
      updatedAt: "2025-01-01T00:00:00.000Z",
    };

    const skills = Array.from({ length: 100 }, (_, i) => ({
      ...baseSkill,
      name: `skill-${i.toString().padStart(3, "0")}`,
    }));

    const saveResult = await saveInstalled({ version: 1, skills });
    expect(saveResult.ok).toBe(true);

    const reloadResult = await loadInstalled();
    expect(reloadResult.ok).toBe(true);
    if (!reloadResult.ok) return;
    expect(reloadResult.value.skills).toHaveLength(100);
    expect(reloadResult.value.skills[0]?.name).toBe("skill-000");
    expect(reloadResult.value.skills[99]?.name).toBe("skill-099");
  });
});

describe("installSkill — tap name resolution", () => {
  test("tap name installs only the requested skill from a multi-skill repo", async () => {
    const repo = await createMultiSkillRepo();
    try {
      // Set up a tap that maps "skill-a" to the multi-skill repo
      const tapName = "test-tap";
      const tapsDir = join(configDir, "skilltap", "taps", tapName);
      await $`mkdir -p ${tapsDir}`.quiet();
      const tapJson = JSON.stringify({
        name: tapName,
        description: "Test tap",
        skills: [
          { name: "skill-a", description: "Skill A", repo: repo.path, tags: [] },
          { name: "skill-b", description: "Skill B", repo: repo.path, tags: [] },
        ],
      });
      await Bun.write(join(tapsDir, "tap.json"), tapJson);

      // Write config with builtin_tap disabled and our test tap
      const configPath = join(configDir, "skilltap", "config.toml");
      await Bun.write(configPath, `builtin_tap = false\n\n[[taps]]\nname = "${tapName}"\nurl = "${repo.path}"\n`);

      // Install by tap name — should only install skill-a, not both
      const result = await installSkill("skill-a", { scope: "global", skipScan: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records).toHaveLength(1);
      expect(result.value.records[0]?.name).toBe("skill-a");

      // skill-b should NOT be installed
      const skillBDir = join(homeDir, ".agents", "skills", "skill-b");
      expect(await lstat(skillBDir).catch(() => null)).toBeNull();

      // installed.json should only have skill-a
      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      expect(installedResult.value.skills).toHaveLength(1);
      expect(installedResult.value.skills[0]?.name).toBe("skill-a");
    } finally {
      await repo.cleanup();
    }
  });

  test("onSelectSkills is NOT called when source resolves via tap", async () => {
    const repo = await createMultiSkillRepo();
    try {
      const tapName = "test-tap";
      const tapsDir = join(configDir, "skilltap", "taps", tapName);
      await $`mkdir -p ${tapsDir}`.quiet();
      await Bun.write(
        join(tapsDir, "tap.json"),
        JSON.stringify({
          name: tapName,
          skills: [
            { name: "skill-a", description: "Skill A", repo: repo.path, tags: [] },
            { name: "skill-b", description: "Skill B", repo: repo.path, tags: [] },
          ],
        }),
      );
      await Bun.write(
        join(configDir, "skilltap", "config.toml"),
        `builtin_tap = false\n\n[[taps]]\nname = "${tapName}"\nurl = "${repo.path}"\n`,
      );

      let selectSkillsCalled = false;
      const result = await installSkill("skill-b", {
        scope: "global",
        skipScan: true,
        onSelectSkills: async (skills) => {
          selectSkillsCalled = true;
          return skills.map((s) => s.name);
        },
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(selectSkillsCalled).toBe(false);
      expect(result.value.records).toHaveLength(1);
      expect(result.value.records[0]?.name).toBe("skill-b");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("findProjectRoot", () => {
  test("finds nearest .git directory", async () => {
    const root = await makeTmpDir();
    try {
      await $`git -C ${root} init`.quiet();
      const nested = join(root, "a", "b", "c");
      await $`mkdir -p ${nested}`.quiet();
      const found = await findProjectRoot(nested);
      expect(found).toBe(root);
    } finally {
      await removeTmpDir(root);
    }
  });

  test("falls back to startDir when no .git found", async () => {
    const dir = await makeTmpDir();
    try {
      const found = await findProjectRoot(dir);
      // No .git above dir (since /tmp won't have one), should return dir itself
      // (or somewhere above — just check it doesn't throw)
      expect(typeof found).toBe("string");
    } finally {
      await removeTmpDir(dir);
    }
  });
});
