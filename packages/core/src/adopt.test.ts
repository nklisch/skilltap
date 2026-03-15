import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { adoptSkill } from "./adopt";
import { discoverSkills } from "./discover";

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

const SKILL_MD = `---
name: my-skill
description: A test skill
---
# My Skill
`;

async function createUnmanagedSkillInDir(
  baseDir: string,
  name: string,
): Promise<string> {
  const skillDir = join(baseDir, name);
  await mkdir(skillDir, { recursive: true });
  await Bun.write(
    join(skillDir, "SKILL.md"),
    `---\nname: ${name}\ndescription: A test skill\n---\n# Skill\n`,
  );
  return skillDir;
}

describe("adoptSkill", () => {
  test("move mode: moves dir to .agents/skills/", async () => {
    // Create skill in .claude/skills (outside .agents/skills)
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await mkdir(claudeSkillsDir, { recursive: true });
    const srcPath = await createUnmanagedSkillInDir(claudeSkillsDir, "my-skill");

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    const result = await adoptSkill(skill, {
      mode: "move",
      scope: "global",
      skipScan: true,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // The skill should now be in .agents/skills/
    const targetPath = join(homeDir, ".agents", "skills", "my-skill");
    const stat = await lstat(targetPath).catch(() => null);
    expect(stat?.isDirectory()).toBe(true);

    // Record should be in installed.json
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills.find((s) => s.name === "my-skill")).toBeDefined();
  });

  test("move mode: creates symlink from original location", async () => {
    // Create skill in .claude/skills
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await mkdir(claudeSkillsDir, { recursive: true });
    await createUnmanagedSkillInDir(claudeSkillsDir, "my-skill");

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    const srcPath = join(claudeSkillsDir, "my-skill");

    await adoptSkill(skill, {
      mode: "move",
      scope: "global",
      skipScan: true,
    });

    // A symlink should exist at the original location
    const stat = await lstat(srcPath).catch(() => null);
    expect(stat).not.toBeNull();
    expect(stat?.isSymbolicLink()).toBe(true);
  });

  test("move mode: skill already at target just creates record", async () => {
    // Create skill directly in .agents/skills (the canonical install dir)
    const agentsSkillsDir = join(homeDir, ".agents", "skills");
    await mkdir(agentsSkillsDir, { recursive: true });
    await createUnmanagedSkillInDir(agentsSkillsDir, "my-skill");

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    const result = await adoptSkill(skill, {
      mode: "move",
      scope: "global",
      skipScan: true,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Skill still in its original location
    const targetPath = join(homeDir, ".agents", "skills", "my-skill");
    const stat = await lstat(targetPath).catch(() => null);
    expect(stat?.isDirectory()).toBe(true);

    // Record created
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills.find((s) => s.name === "my-skill")).toBeDefined();
  });

  test("track-in-place mode: creates linked record", async () => {
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await mkdir(claudeSkillsDir, { recursive: true });
    const srcPath = await createUnmanagedSkillInDir(claudeSkillsDir, "my-skill");

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    const result = await adoptSkill(skill, {
      mode: "track-in-place",
      scope: "global",
      skipScan: true,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Record should be "linked" scope with path
    expect(result.value.record.scope).toBe("linked");
    expect(result.value.record.path).toBe(srcPath);

    // Skill still at original location
    const stat = await lstat(srcPath).catch(() => null);
    expect(stat?.isDirectory()).toBe(true);

    // NOT moved to .agents/skills/
    const targetPath = join(homeDir, ".agents", "skills", "my-skill");
    const targetStat = await lstat(targetPath).catch(() => null);
    expect(targetStat).toBeNull();
  });

  test("errors on already-managed skill", async () => {
    const agentsSkillsDir = join(homeDir, ".agents", "skills");
    await mkdir(agentsSkillsDir, { recursive: true });
    await createUnmanagedSkillInDir(agentsSkillsDir, "my-skill");

    // Write a managed record
    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "my-skill",
          description: "A test skill",
          repo: null,
          ref: null,
          sha: null,
          scope: "global",
          path: null,
          tap: null,
          also: [],
          installedAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
    });

    const discoverResult = await discoverSkills({ global: true, project: false });
    expect(discoverResult.ok).toBe(true);
    if (!discoverResult.ok) return;

    const skill = discoverResult.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    const result = await adoptSkill(skill, { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already managed");
  });
});
