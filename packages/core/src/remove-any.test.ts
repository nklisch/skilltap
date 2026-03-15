import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, mkdir, symlink } from "node:fs/promises";
import { join } from "node:path";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { loadInstalled } from "./config";
import { discoverSkills } from "./discover";
import { installSkill } from "./install";
import { removeAnySkill } from "./remove";

setDefaultTimeout(30_000);

type Env = { SKILLTAP_HOME?: string; XDG_CONFIG_HOME?: string };

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

describe("removeAnySkill", () => {
  test("delegates to removeSkill for managed skills", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      // Install the skill so it's managed
      const installResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(installResult.ok).toBe(true);

      // The install dir should exist
      const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
      const beforeStat = await lstat(skillDir).catch(() => null);
      expect(beforeStat?.isDirectory()).toBe(true);

      // Discover the skill
      const discoverResult = await discoverSkills({ global: true, project: false });
      expect(discoverResult.ok).toBe(true);
      if (!discoverResult.ok) return;

      const skill = discoverResult.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(skill).toBeDefined();
      if (!skill) return;
      expect(skill.managed).toBe(true);

      // Remove via removeAnySkill
      const result = await removeAnySkill({ skill, removeAll: true });
      expect(result.ok).toBe(true);

      // Skill directory should be gone
      const afterStat = await lstat(skillDir).catch(() => null);
      expect(afterStat).toBeNull();

      // installed.json should no longer have the record
      const loaded = await loadInstalled();
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      expect(
        loaded.value.skills.find((s) => s.name === "standalone-skill"),
      ).toBeUndefined();
    } finally {
      await repo.cleanup();
    }
  });

  test("removes unmanaged directory", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "unmanaged-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: unmanaged-skill\ndescription: Unmanaged\n---\n# Unmanaged Skill\n`,
    );

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find(
      (s) => s.name === "unmanaged-skill",
    );
    expect(skill).toBeDefined();
    if (!skill) return;
    expect(skill.managed).toBe(false);

    const result = await removeAnySkill({ skill, removeAll: true });
    expect(result.ok).toBe(true);

    // Directory should be gone
    const afterStat = await lstat(skillDir).catch(() => null);
    expect(afterStat).toBeNull();
  });

  test("only unlinks symlinks without deleting the target", async () => {
    // Create a real directory in a separate tmp location (simulates an external skill)
    const externalDir = await makeTmpDir();
    try {
      const realSkillDir = join(externalDir, "real-skill");
      await mkdir(realSkillDir, { recursive: true });
      await Bun.write(
        join(realSkillDir, "SKILL.md"),
        `---\nname: real-skill\ndescription: Real skill\n---\n# Real Skill\n`,
      );

      // Create a symlink in the agent-specific dir pointing to the real dir
      const claudeSkillsDir = join(homeDir, ".claude", "skills");
      await mkdir(claudeSkillsDir, { recursive: true });
      const linkPath = join(claudeSkillsDir, "real-skill");
      await symlink(realSkillDir, linkPath, "dir");

      const discoverResult = await discoverSkills({ global: true, project: false });
      expect(discoverResult.ok).toBe(true);
      if (!discoverResult.ok) return;

      const skill = discoverResult.value.skills.find(
        (s) => s.name === "real-skill",
      );
      expect(skill).toBeDefined();
      if (!skill) return;

      // Find the symlink location
      const symlinkLoc = skill.locations.find((l) => l.isSymlink);
      expect(symlinkLoc).toBeDefined();
      if (!symlinkLoc) return;

      // Remove only the symlink location
      const result = await removeAnySkill({ skill, locations: [symlinkLoc] });
      expect(result.ok).toBe(true);

      // The symlink should be gone
      const linkStat = await lstat(linkPath).catch(() => null);
      expect(linkStat).toBeNull();

      // But the real directory should still exist
      const realStat = await lstat(realSkillDir).catch(() => null);
      expect(realStat?.isDirectory()).toBe(true);
    } finally {
      await removeTmpDir(externalDir);
    }
  });
});
