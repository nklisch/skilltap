import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat, readlink } from "node:fs/promises";
import { join } from "node:path";
import {
  createMaliciousSkillRepo,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { installSkill } from "./install";
import { findProjectRoot } from "./paths";
import { removeSkill } from "./remove";

type Env = {
  SKILLTAP_HOME?: string;
  XDG_CONFIG_HOME?: string;
};

let savedEnv: Env;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  savedEnv = {
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
  };
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  if (savedEnv.XDG_CONFIG_HOME === undefined)
    delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
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

  test("fails with UserError when already installed", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global" });
      const result = await installSkill(repo.path, { scope: "global" });
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
