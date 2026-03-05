import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { installSkill, loadInstalled } from "@skilltap/core";
import {
  createMultiSkillRepo,
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

describe("remove — multiple names", () => {
  test("removes multiple skills with --yes", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const { exitCode, stdout } = await runSkilltap(
        ["remove", "skill-a", "skill-b", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("2 skills");

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("exits 1 if any name not found", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const { exitCode, stderr } = await runSkilltap(
        ["remove", "standalone-skill", "nonexistent", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("not installed");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("remove — not found", () => {
  test("exits 1 with error message", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["remove", "nonexistent", "--yes"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});

describe("remove — with --yes flag", () => {
  test("removes the skill without prompt", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const { exitCode, stdout } = await runSkilltap(
        ["remove", "standalone-skill", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");

      // Verify actually removed
      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("prints success message after removal", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runSkilltap(
        ["remove", "standalone-skill", "--yes"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Removed");
    } finally {
      await repo.cleanup();
    }
  });
});
