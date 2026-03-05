import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, readlink } from "node:fs/promises";
import { join } from "node:path";
import { loadInstalled } from "@skilltap/core";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

setDefaultTimeout(45_000);

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

describe("link — global scope", () => {
  test("creates symlink at install path", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["link", repo.path, "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Linked");
      expect(stdout).toContain("standalone-skill");

      const symlinkPath = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      const stat = await lstat(symlinkPath);
      expect(stat.isSymbolicLink()).toBe(true);

      const target = await readlink(symlinkPath);
      expect(target).toBe(repo.path);
    } finally {
      await repo.cleanup();
    }
  });

  test("records skill with scope=linked in installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await runSkilltap(["link", repo.path, "--global"], homeDir, configDir);

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;

      const skill = installed.value.skills.find(
        (s) => s.name === "standalone-skill",
      );
      expect(skill?.scope).toBe("linked");
      expect(skill?.repo).toBeNull();
      expect(skill?.sha).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("fails when path has no SKILL.md", async () => {
    const tmpDir = await makeTmpDir();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["link", tmpDir, "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("SKILL.md");
    } finally {
      await removeTmpDir(tmpDir);
    }
  });
});
