import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat } from "node:fs/promises";
import { join } from "node:path";
import {
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  createTestEnv,
  makeTmpDir,
  removeTmpDir,
  type TestEnv,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { disableSkill } from "./disable";
import { installSkill } from "./install";
import { skillDisabledDir } from "./paths";
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

describe("removeSkill — global skill", () => {
  test("removes directory and record from installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);

      const result = await removeSkill("standalone-skill", { scope: "global" });
      expect(result.ok).toBe(true);

      expect(await lstat(skillDir).catch(() => null)).toBeNull();

      const loaded = await loadInstalled();
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      expect(loaded.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("removes agent symlinks as part of removal", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
      });
      const linkPath = join(homeDir, ".claude", "skills", "standalone-skill");
      expect(await lstat(linkPath).catch(() => null)).not.toBeNull();

      await removeSkill("standalone-skill", { scope: "global" });
      expect(await lstat(linkPath).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });
});

describe("removeSkill — project skill", () => {
  test("removes project-scoped skill from correct install path", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();
      await installSkill(repo.path, {
        scope: "project",
        projectRoot,
        skipScan: true,
      });
      const skillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);

      const result = await removeSkill("standalone-skill", {
        scope: "project",
        projectRoot,
      });
      expect(result.ok).toBe(true);

      expect(await lstat(skillDir).catch(() => null)).toBeNull();

      const loaded = await loadInstalled();
      if (!loaded.ok) return;
      expect(loaded.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });
});

describe("removeSkill — linked skill", () => {
  test("deletes symlink at record.path, not computed install path", async () => {
    const targetDir = await makeTmpDir();
    const linkParent = await makeTmpDir();
    const symlinkPath = join(linkParent, "linked-skill");
    try {
      // Create an actual symlink
      await $`ln -s ${targetDir} ${symlinkPath}`.quiet();

      // Write linked skill record directly
      await saveInstalled({
        version: 1,
        skills: [
          {
            name: "linked-skill",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "linked",
            path: symlinkPath,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ],
      });

      expect(await lstat(symlinkPath).catch(() => null)).not.toBeNull();

      const result = await removeSkill("linked-skill");
      expect(result.ok).toBe(true);

      // Symlink removed (at record.path)
      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();
      // Target dir itself is untouched
      expect(await lstat(targetDir).then((s) => s.isDirectory())).toBe(true);
    } finally {
      await removeTmpDir(targetDir);
      await removeTmpDir(linkParent);
    }
  });
});

describe("removeSkill — error cases", () => {
  test("returns UserError when skill name not found", async () => {
    const result = await removeSkill("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });

  test("returns UserError when name matches but scope filter doesn't", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Name matches but scope filter is "project" — should not find it
      const result = await removeSkill("standalone-skill", {
        scope: "project",
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("not installed");

      // Skill still installed
      const loaded = await loadInstalled();
      if (!loaded.ok) return;
      expect(loaded.value.skills).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("removeSkill — disabled skill", () => {
  test("removes a disabled skill (finds files in .disabled/, clears record)", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Disable it — moves files to .disabled/standalone-skill
      const disableResult = await disableSkill("standalone-skill");
      expect(disableResult.ok).toBe(true);

      const disabledDir = skillDisabledDir("standalone-skill", "global");
      expect(await lstat(disabledDir).then((s) => s.isDirectory())).toBe(true);

      // Remove the (disabled) skill
      const result = await removeSkill("standalone-skill", { scope: "global" });
      expect(result.ok).toBe(true);

      // .disabled/<name> directory is gone
      expect(await lstat(disabledDir).catch(() => null)).toBeNull();

      // Record removed from installed.json
      const loaded = await loadInstalled();
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      expect(loaded.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("removeSkill — cache cleanup", () => {
  test("removes cache dir when last skill from that repo is removed", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, {
        scope: "global",
        skillNames: ["skill-a"],
        skipScan: true,
      });
      const loaded = await loadInstalled();
      if (!loaded.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: install succeeded
      const record = loaded.value.skills[0]!;
      // biome-ignore lint/style/noNonNullAssertion: multi-skill always sets repo
      const hash = Bun.hash(record.repo!).toString(16);
      const cacheRoot = join(configDir, "skilltap", "cache", hash);
      expect(await lstat(cacheRoot).then((s) => s.isDirectory())).toBe(true);

      await removeSkill("skill-a", { scope: "global" });
      expect(await lstat(cacheRoot).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("preserves cache dir when other skills from same repo remain", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const loaded = await loadInstalled();
      if (!loaded.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: install succeeded
      const record = loaded.value.skills[0]!;
      // biome-ignore lint/style/noNonNullAssertion: multi-skill always sets repo
      const hash = Bun.hash(record.repo!).toString(16);
      const cacheRoot = join(configDir, "skilltap", "cache", hash);

      await removeSkill("skill-a", { scope: "global" });
      // Cache still exists because skill-b is from the same repo
      expect(await lstat(cacheRoot).then((s) => s.isDirectory())).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});
