import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, mkdir, readlink } from "node:fs/promises";
import { join } from "node:path";
import { loadInstalled } from "@skilltap/core";
import { createTestEnv, type TestEnv, createStandaloneSkillRepo, runSkilltap, makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

setDefaultTimeout(60_000);

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

describe("skills link — global scope", () => {
  test("creates symlink at install path", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "link", repo.path, "--global"],
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
      await runSkilltap(["skills", "link", repo.path, "--global"], homeDir, configDir);

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
        ["skills", "link", tmpDir, "--global"],
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

// ── Helper ─────────────────────────────────────────────────────────────────

async function writeConfig(toml: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), toml);
}

// ── Agent symlinks (--also and config defaults.also) ───────────────────────
// SPEC.md: --also default is "(from config)"; "Create agent symlinks if --also"

describe("skills link — agent symlinks", () => {
  test("--also flag creates agent symlink", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "link", repo.path, "--global", "--also", "claude-code"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Also linked for claude-code");

      const agentSymlink = join(homeDir, ".claude", "skills", "standalone-skill");
      const stat = await lstat(agentSymlink);
      expect(stat.isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("config defaults.also creates agent symlinks when --also not passed", async () => {
    await writeConfig('[defaults]\nalso = ["claude-code"]\n');
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "link", repo.path, "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Also linked for claude-code");

      const agentSymlink = join(homeDir, ".claude", "skills", "standalone-skill");
      const stat = await lstat(agentSymlink);
      expect(stat.isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("no --also and no config creates no agent symlinks", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["skills", "link", repo.path, "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const agentSymlink = join(homeDir, ".claude", "skills", "standalone-skill");
      expect(await lstat(agentSymlink).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("--also with multiple agents creates all symlinks", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["skills", "link", repo.path, "--global", "--also", "claude-code,cursor"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const claudeLink = join(homeDir, ".claude", "skills", "standalone-skill");
      const cursorLink = join(homeDir, ".cursor", "skills", "standalone-skill");
      expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);
      expect((await lstat(cursorLink)).isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("--also overrides config defaults.also", async () => {
    await writeConfig('[defaults]\nalso = ["cursor"]\n');
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "link", repo.path, "--global", "--also", "claude-code"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Also linked for claude-code");
      expect(stdout).not.toContain("cursor");

      // Only claude-code should exist, not cursor
      const claudeLink = join(homeDir, ".claude", "skills", "standalone-skill");
      const cursorLink = join(homeDir, ".cursor", "skills", "standalone-skill");
      expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);
      expect(await lstat(cursorLink).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("--also with invalid agent exits with code 1", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["skills", "link", repo.path, "--global", "--also", "invalid-agent"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("Unknown agent");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("aliases", () => {
  test("skilltap link routes to skills link", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["link", repo.path, "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Linked");
    } finally {
      await repo.cleanup();
    }
  });
});
