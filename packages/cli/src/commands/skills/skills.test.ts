import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { installSkill, loadInstalled } from "@skilltap/core";
import { $ } from "bun";
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

async function createUnmanagedSkill(home: string, name: string) {
  const skillDir = join(home, ".agents", "skills", name);
  await mkdir(skillDir, { recursive: true });
  await Bun.write(
    join(skillDir, "SKILL.md"),
    `---\nname: ${name}\ndescription: An unmanaged skill\n---\n# ${name}\n`,
  );
  return skillDir;
}

describe("skilltap skills", () => {
  test("shows empty state message when no skills", async () => {
    const { exitCode, stdout } = await runSkilltap(["skills"], homeDir, configDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No skills found");
  });

  test("--project shows only project skills", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();

      // Install global skill
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Create an unmanaged project-scoped skill
      const projectSkillDir = join(projectRoot, ".agents", "skills", "project-only");
      await mkdir(projectSkillDir, { recursive: true });
      await Bun.write(
        join(projectSkillDir, "SKILL.md"),
        `---\nname: project-only\ndescription: A project skill\n---\n# Project Only\n`,
      );

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "--project"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("project-only");
      expect(stdout).not.toContain("standalone-skill");
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });

  test("shows unified view with managed and unmanaged skills", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await createUnmanagedSkill(homeDir, "unmanaged-skill");

      const { exitCode, stdout } = await runSkilltap(["skills"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
      expect(stdout).toContain("unmanaged-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("--unmanaged filters to unmanaged only", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await createUnmanagedSkill(homeDir, "unmanaged-skill");

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "--unmanaged"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("unmanaged-skill");
      expect(stdout).not.toContain("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("--json outputs valid JSON array", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "--json"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      const parsed = JSON.parse(stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.some((s: { name: string }) => s.name === "standalone-skill")).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("skilltap skills adopt", () => {
  test("adopts an unmanaged skill by name", async () => {
    await createUnmanagedSkill(homeDir, "adopt-me");

    const { exitCode, stdout } = await runSkilltap(
      ["skills", "adopt", "adopt-me", "--global", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("Adopted");
    expect(stdout).toContain("adopt-me");
  });

  test("--track-in-place creates linked record", async () => {
    const skillDir = join(homeDir, ".claude", "skills", "track-me");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: track-me\ndescription: A track-in-place skill\n---\n# Track Me\n`,
    );

    const { exitCode, stdout } = await runSkilltap(
      ["skills", "adopt", "track-me", "--global", "--track-in-place", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("track-me");

    // Verify installed.json has a linked record
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    const record = loaded.value.skills.find((s) => s.name === "track-me");
    expect(record).toBeDefined();
    expect(record?.scope).toBe("linked");
  });
});

describe("skilltap skills move", () => {
  test("errors without scope flag", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stderr } = await runSkilltap(
        ["skills", "move", "standalone-skill"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("scope");
    } finally {
      await repo.cleanup();
    }
  });

  test("errors when skill not found", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["skills", "move", "nonexistent", "--global"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});
