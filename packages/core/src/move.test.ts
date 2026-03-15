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
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled } from "./config";
import { installSkill } from "./install";
import { moveSkill } from "./move";

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

describe("moveSkill", () => {
  test("moves from global to project", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();

      // Install globally
      const installResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(installResult.ok).toBe(true);

      const globalSkillDir = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(globalSkillDir)
          .then((s) => s.isDirectory())
          .catch(() => false),
      ).toBe(true);

      // Move to project
      const moveResult = await moveSkill("standalone-skill", {
        to: { scope: "project", projectRoot },
      });

      expect(moveResult.ok).toBe(true);
      if (!moveResult.ok) return;

      // Should now be in project scope
      const projectSkillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(projectSkillDir)
          .then((s) => s.isDirectory())
          .catch(() => false),
      ).toBe(true);

      // Should NOT be in global scope
      expect(await lstat(globalSkillDir).catch(() => null)).toBeNull();

      // Project installed.json should have record
      const projectInstalled = await loadInstalled(projectRoot);
      expect(projectInstalled.ok).toBe(true);
      if (!projectInstalled.ok) return;
      const projectRecord = projectInstalled.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(projectRecord).toBeDefined();
      expect(projectRecord?.scope).toBe("project");

      // Global installed.json should NOT have record
      const globalInstalled = await loadInstalled();
      expect(globalInstalled.ok).toBe(true);
      if (!globalInstalled.ok) return;
      expect(
        globalInstalled.value.skills.find((s) => s.name === "standalone-skill"),
      ).toBeUndefined();
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("moves from project to global", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();

      // Install to project
      const installResult = await installSkill(repo.path, {
        scope: "project",
        projectRoot,
        skipScan: true,
      });
      expect(installResult.ok).toBe(true);

      const projectSkillDir = join(
        projectRoot,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(projectSkillDir)
          .then((s) => s.isDirectory())
          .catch(() => false),
      ).toBe(true);

      // Move to global (specify fromProjectRoot so the source can be located)
      const moveResult = await moveSkill("standalone-skill", {
        to: { scope: "global" },
        fromProjectRoot: projectRoot,
      });

      expect(moveResult.ok).toBe(true);
      if (!moveResult.ok) return;

      // Should now be in global scope
      const globalSkillDir = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(globalSkillDir)
          .then((s) => s.isDirectory())
          .catch(() => false),
      ).toBe(true);

      // Should NOT be in project scope
      expect(await lstat(projectSkillDir).catch(() => null)).toBeNull();

      // Global installed.json should have record
      const globalInstalled = await loadInstalled();
      expect(globalInstalled.ok).toBe(true);
      if (!globalInstalled.ok) return;
      const globalRecord = globalInstalled.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(globalRecord).toBeDefined();
      expect(globalRecord?.scope).toBe("global");
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("merges existing also with new also option", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();

      // Install globally with also: ["claude-code"]
      const installResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
        also: ["claude-code"],
      });
      expect(installResult.ok).toBe(true);

      // Move to project, adding "cursor" to also
      const moveResult = await moveSkill("standalone-skill", {
        to: { scope: "project", projectRoot },
        also: ["cursor"],
      });

      expect(moveResult.ok).toBe(true);
      if (!moveResult.ok) return;

      // The merged also should contain both agents
      expect(moveResult.value.record.also).toContain("claude-code");
      expect(moveResult.value.record.also).toContain("cursor");
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("errors if already in target scope", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const result = await moveSkill("standalone-skill", {
        to: { scope: "global" },
      });

      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("already in global scope");
    } finally {
      await repo.cleanup();
    }
  });

  test("errors if skill not found", async () => {
    const result = await moveSkill("nonexistent-skill", {
      to: { scope: "global" },
    });

    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });
});
