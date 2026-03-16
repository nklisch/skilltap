import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { join } from "node:path";
import { mkdir } from "node:fs/promises";
import { installSkill, loadInstalled } from "@skilltap/core";
import {
  createMultiSkillRepo,
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

describe("skills remove — multiple names", () => {
  test("removes multiple skills with --yes", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "remove", "skill-a", "skill-b", "--yes"],
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
        ["skills", "remove", "standalone-skill", "nonexistent", "--yes"],
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

describe("skills remove — not found", () => {
  test("exits 1 with error message", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["skills", "remove", "nonexistent", "--yes"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});

describe("skills remove — with --yes flag", () => {
  test("removes the skill without prompt", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "remove", "standalone-skill", "--yes"],
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
        ["skills", "remove", "standalone-skill", "--yes"],
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

describe("aliases", () => {
  test("skilltap remove routes to skills remove", async () => {
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

describe("skills remove — unmanaged skill", () => {
  test("removes an unmanaged skill from disk with --yes", async () => {
    // Create unmanaged skill in global .agents/skills/
    const skillDir = join(homeDir, ".agents", "skills", "unmanaged-test");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      "---\nname: unmanaged-test\ndescription: An unmanaged skill\n---\n# Test\n",
    );

    const { exitCode, stdout } = await runSkilltap(
      ["skills", "remove", "unmanaged-test", "--yes"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("Removed");
  });
});
