import { lstat, mkdir, readlink } from "node:fs/promises";
import { join } from "node:path";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";

setDefaultTimeout(60_000);
import { loadInstalled } from "@skilltap/core";
import {
  createMaliciousSkillRepo,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  createTestEnv,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

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

describe("install — standalone skill", () => {
  test("installs with --yes --global and shows success", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(1);
      expect(installed.value.skills[0]?.name).toBe("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("auto-updates with --yes when already installed", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("install — multi-skill repo", () => {
  test("auto-selects all skills with --yes", async () => {
    const repo = await createMultiSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const installed = await loadInstalled();
      expect(installed.ok).toBe(true);
      if (!installed.ok) return;
      expect(installed.value.skills).toHaveLength(2);
      const names = installed.value.skills.map((s) => s.name).sort();
      expect(names).toEqual(["skill-a", "skill-b"]);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("install — security scanning", () => {
  test("--skip-scan bypasses security check", async () => {
    const repo = await createMaliciousSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("--strict aborts on warnings from malicious skill", async () => {
    const repo = await createMaliciousSkillRepo();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--strict"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("aborting");
    } finally {
      await repo.cleanup();
    }
  });
});

// ── Agent Selection Tests ──

async function writeConfig(
  configDir: string,
  toml: string,
): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), toml);
}

describe("install — agent selection", () => {
  test("--yes uses config defaults.also for symlinks", async () => {
    await writeConfig(
      configDir,
      '[defaults]\nalso = ["claude-code"]\n',
    );
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      // Verify the agent symlink was created
      const symlinkPath = join(
        homeDir,
        ".claude",
        "skills",
        "standalone-skill",
      );
      const stat = await lstat(symlinkPath);
      expect(stat.isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("--also flag creates symlink and skips prompt", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan", "--also", "claude-code"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const symlinkPath = join(
        homeDir,
        ".claude",
        "skills",
        "standalone-skill",
      );
      const stat = await lstat(symlinkPath);
      expect(stat.isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("--yes without config defaults.also creates no symlinks", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      // No agent symlink should exist
      const symlinkPath = join(
        homeDir,
        ".claude",
        "skills",
        "standalone-skill",
      );
      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();
    } finally {
      await repo.cleanup();
    }
  });

  test("--also with multiple agents creates all symlinks", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan", "--also", "claude-code,cursor"],
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

  test("config defaults.also with multiple agents creates all symlinks", async () => {
    await writeConfig(
      configDir,
      '[defaults]\nalso = ["claude-code", "cursor"]\n',
    );
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path, "--yes", "--global", "--skip-scan"],
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
});

// ── Agent Mode Tests ──

async function writeAgentModeConfig(
  configDir: string,
  extra = "",
): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(
    join(dir, "config.toml"),
    `["agent-mode"]\nenabled = true\nscope = "global"\n\n[security]\nscan = "static"\n${extra}`,
  );
}

describe("install — agent mode", () => {
  test("clean skill installs with plain text output", async () => {
    await writeAgentModeConfig(configDir);
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout, stderr } = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("OK: Installed standalone-skill");
      // No ANSI escape codes
      expect(stdout).not.toMatch(/\x1b\[/);
      expect(stderr).not.toMatch(/\x1b\[/);
    } finally {
      await repo.cleanup();
    }
  });

  test("malicious skill blocked with security directive", async () => {
    await writeAgentModeConfig(configDir);
    const repo = await createMaliciousSkillRepo();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("SECURITY ISSUE FOUND");
      expect(stderr).toContain("DO NOT install");
      expect(stderr).toContain("User action required");
    } finally {
      await repo.cleanup();
    }
  });

  test("--skip-scan blocked in agent mode", async () => {
    await writeAgentModeConfig(configDir);
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["install", repo.path, "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("Agent mode");
      expect(stderr).toContain("--skip-scan");
    } finally {
      await repo.cleanup();
    }
  });

  test("auto-selects all skills from multi-skill repo", async () => {
    await writeAgentModeConfig(configDir);
    const repo = await createMultiSkillRepo();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("OK: Installed skill-a");
      expect(stdout).toContain("OK: Installed skill-b");
    } finally {
      await repo.cleanup();
    }
  });

  test("already installed triggers update instead of failing", async () => {
    await writeAgentModeConfig(configDir);
    const repo = await createStandaloneSkillRepo();
    try {
      await runSkilltap(["install", repo.path], homeDir, configDir);
      const { exitCode, stdout } = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toMatch(/up to date|Updated/i);
    } finally {
      await repo.cleanup();
    }
  });

  test("agent mode uses config defaults.also for symlinks", async () => {
    await writeAgentModeConfig(
      configDir,
      '\n[defaults]\nalso = ["claude-code"]\n',
    );
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode } = await runSkilltap(
        ["install", repo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const symlinkPath = join(
        homeDir,
        ".claude",
        "skills",
        "standalone-skill",
      );
      const stat = await lstat(symlinkPath);
      expect(stat.isSymbolicLink()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});
