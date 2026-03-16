import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { linkSkill, loadInstalled } from "@skilltap/core";
import {
  createStandaloneSkillRepo,
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

describe("skills unlink — not found", () => {
  test("exits 1 when skill not linked", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["skills", "unlink", "nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not linked");
  });
});

describe("skills unlink — linked skill", () => {
  test("removes the symlink", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await linkSkill(repo.path, { scope: "global" });

      const symlinkPath = join(
        homeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      expect(
        await lstat(symlinkPath)
          .then(() => true)
          .catch(() => false),
      ).toBe(true);

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "unlink", "standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Unlinked");

      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("removes skill from installed.json", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await linkSkill(repo.path, { scope: "global" });
      await runSkilltap(["skills", "unlink", "standalone-skill"], homeDir, configDir);

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("aliases", () => {
  test("skilltap unlink routes to skills unlink", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await linkSkill(repo.path, { scope: "global" });
      const { exitCode, stdout } = await runSkilltap(
        ["unlink", "standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Unlinked");
    } finally {
      await repo.cleanup();
    }
  });
});
