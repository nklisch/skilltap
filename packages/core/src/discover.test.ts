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
import { createTestEnv, makeTmpDir, removeTmpDir, type TestEnv } from "@skilltap/test-utils";
import { $ } from "bun";
import { saveInstalled } from "./config";
import { discoverSkills } from "./discover";

setDefaultTimeout(45_000);

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

const SKILL_MD = `---
name: test-skill
description: A test skill
---
# Test Skill
`;

describe("discoverSkills", () => {
  test("discovers skills in .agents/skills/", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "test-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), SKILL_MD);

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "test-skill");
    expect(skill).toBeDefined();
    expect(skill?.managed).toBe(false);
    expect(skill?.description).toBe("A test skill");
    expect(skill?.locations).toHaveLength(1);
    expect(skill?.locations[0]?.source.type).toBe("agents");
  });

  test("discovers skills in agent-specific dirs", async () => {
    const skillDir = join(homeDir, ".claude", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: my-skill\ndescription: My skill\n---\n# My Skill\n`,
    );

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "my-skill");
    expect(skill).toBeDefined();
    expect(skill?.locations[0]?.source.type).toBe("agent-specific");
    const src = skill?.locations[0]?.source;
    if (src?.type === "agent-specific") {
      expect(src.agent).toBe("claude-code");
    }
  });

  test("deduplicates symlinked skills", async () => {
    const realDir = join(homeDir, ".agents", "skills", "foo");
    await mkdir(realDir, { recursive: true });
    await Bun.write(
      join(realDir, "SKILL.md"),
      `---\nname: foo\ndescription: Foo skill\n---\n# Foo\n`,
    );

    // Create .claude/skills dir and symlink
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await mkdir(claudeSkillsDir, { recursive: true });
    const linkPath = join(claudeSkillsDir, "foo");
    await symlink(realDir, linkPath, "dir");

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const fooSkills = result.value.skills.filter((s) => s.name === "foo");
    // Should be deduplicated to a single DiscoveredSkill
    expect(fooSkills).toHaveLength(1);
    // Should have 2 locations: real dir + symlink
    expect(fooSkills[0]?.locations).toHaveLength(2);
    const symlinkLoc = fooSkills[0]?.locations.find((l) => l.isSymlink);
    expect(symlinkLoc).toBeDefined();
  });

  test("marks managed skills from installed.json", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "managed-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: managed-skill\ndescription: Managed\n---\n`,
    );

    // Write a record into installed.json
    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "managed-skill",
          description: "Managed",
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

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "managed-skill");
    expect(skill?.managed).toBe(true);
    expect(skill?.record).not.toBeNull();
    expect(result.value.managed).toBe(1);
    expect(result.value.unmanaged).toBe(0);
  });

  test("marks unmanaged skills without records", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "unmanaged-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: unmanaged-skill\ndescription: Unmanaged\n---\n`,
    );

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "unmanaged-skill");
    expect(skill?.managed).toBe(false);
    expect(skill?.record).toBeNull();
    expect(result.value.unmanaged).toBe(1);
  });

  test("respects unmanagedOnly filter", async () => {
    // Create a managed skill
    const managedDir = join(homeDir, ".agents", "skills", "managed");
    await mkdir(managedDir, { recursive: true });
    await Bun.write(
      join(managedDir, "SKILL.md"),
      `---\nname: managed\ndescription: Managed skill\n---\n`,
    );
    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "managed",
          description: "Managed skill",
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

    // Create an unmanaged skill
    const unmanagedDir = join(homeDir, ".agents", "skills", "unmanaged");
    await mkdir(unmanagedDir, { recursive: true });
    await Bun.write(
      join(unmanagedDir, "SKILL.md"),
      `---\nname: unmanaged\ndescription: Unmanaged skill\n---\n`,
    );

    const result = await discoverSkills({
      global: true,
      project: false,
      unmanagedOnly: true,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.skills.every((s) => !s.managed)).toBe(true);
    expect(result.value.skills.find((s) => s.name === "managed")).toBeUndefined();
    expect(result.value.skills.find((s) => s.name === "unmanaged")).toBeDefined();
  });

  test("respects global/project filters — global only", async () => {
    const globalSkillDir = join(homeDir, ".agents", "skills", "global-skill");
    await mkdir(globalSkillDir, { recursive: true });
    await Bun.write(
      join(globalSkillDir, "SKILL.md"),
      `---\nname: global-skill\ndescription: Global skill\n---\n`,
    );

    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();
      const projectSkillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "project-skill",
      );
      await mkdir(projectSkillDir, { recursive: true });
      await Bun.write(
        join(projectSkillDir, "SKILL.md"),
        `---\nname: project-skill\ndescription: Project skill\n---\n`,
      );

      // global: true, project: false — only global scope
      const result = await discoverSkills({
        global: true,
        project: false,
        projectRoot,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(
        result.value.skills.find((s) => s.name === "global-skill"),
      ).toBeDefined();
      expect(
        result.value.skills.find((s) => s.name === "project-skill"),
      ).toBeUndefined();
    } finally {
      await removeTmpDir(projectRoot);
    }
  });

  test("respects global/project filters — project only", async () => {
    const globalSkillDir = join(homeDir, ".agents", "skills", "global-skill");
    await mkdir(globalSkillDir, { recursive: true });
    await Bun.write(
      join(globalSkillDir, "SKILL.md"),
      `---\nname: global-skill\ndescription: Global skill\n---\n`,
    );

    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();
      const projectSkillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "project-skill",
      );
      await mkdir(projectSkillDir, { recursive: true });
      await Bun.write(
        join(projectSkillDir, "SKILL.md"),
        `---\nname: project-skill\ndescription: Project skill\n---\n`,
      );

      // global: false, project: true — only project scope
      const result = await discoverSkills({
        global: false,
        project: true,
        projectRoot,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(
        result.value.skills.find((s) => s.name === "global-skill"),
      ).toBeUndefined();
      expect(
        result.value.skills.find((s) => s.name === "project-skill"),
      ).toBeDefined();
    } finally {
      await removeTmpDir(projectRoot);
    }
  });

  test("detects git remote on unmanaged skill", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "git-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: git-skill\ndescription: A git skill\n---\n# Git Skill\n`,
    );
    await $`git -C ${skillDir} init`.quiet();
    await $`git -C ${skillDir} remote add origin https://github.com/test/repo.git`.quiet();

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "git-skill");
    expect(skill).toBeDefined();
    expect(skill?.gitRemote).toBe("https://github.com/test/repo.git");
  });

  test("populates symlinkTarget for symlinked locations", async () => {
    const realDir = join(homeDir, ".agents", "skills", "foo");
    await mkdir(realDir, { recursive: true });
    await Bun.write(
      join(realDir, "SKILL.md"),
      `---\nname: foo\ndescription: Foo skill\n---\n# Foo\n`,
    );

    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await mkdir(claudeSkillsDir, { recursive: true });
    const linkPath = join(claudeSkillsDir, "foo");
    await symlink(realDir, linkPath, "dir");

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "foo");
    expect(skill).toBeDefined();
    if (!skill) return;

    const symlinkLoc = skill.locations.find((l) => l.isSymlink);
    expect(symlinkLoc).toBeDefined();
    expect(symlinkLoc?.symlinkTarget).toBe(realDir);
  });

  test("returns empty result when no skills exist", async () => {
    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.skills).toEqual([]);
    expect(result.value.managed).toBe(0);
    expect(result.value.unmanaged).toBe(0);
  });

  test("parses description from SKILL.md without frontmatter", async () => {
    const skillDir = join(homeDir, ".agents", "skills", "no-frontmatter");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), `# My Skill\n\nSome content.\n`);

    const result = await discoverSkills({ global: true, project: false });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const skill = result.value.skills.find((s) => s.name === "no-frontmatter");
    expect(skill).toBeDefined();
    expect(skill?.description).toBe("");
  });
});
