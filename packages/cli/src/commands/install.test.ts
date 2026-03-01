import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { loadInstalled } from "@skilltap/core";
import {
  createMaliciousSkillRepo,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runInstall(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "install", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

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

describe("install — standalone skill", () => {
  test("installs with --yes --global and shows success", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      const { exitCode, stdout } = await runInstall(
        [repo.path, "--yes", "--global", "--skip-scan"],
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

  test("exits 1 when already installed", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await runInstall(
        [repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      const { exitCode, stderr } = await runInstall(
        [repo.path, "--yes", "--global", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("already installed");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("install — multi-skill repo", () => {
  test("auto-selects all skills with --yes", async () => {
    const repo = await createMultiSkillRepo();
    try {
      const { exitCode } = await runInstall(
        [repo.path, "--yes", "--global", "--skip-scan"],
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
      const { exitCode } = await runInstall(
        [repo.path, "--yes", "--global", "--skip-scan"],
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
      const { exitCode, stderr } = await runInstall(
        [repo.path, "--yes", "--global", "--strict"],
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
      const { exitCode, stdout, stderr } = await runInstall(
        [repo.path],
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
      const { exitCode, stderr } = await runInstall(
        [repo.path],
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
      const { exitCode, stderr } = await runInstall(
        [repo.path, "--skip-scan"],
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
      const { exitCode, stdout } = await runInstall(
        [repo.path],
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
});
