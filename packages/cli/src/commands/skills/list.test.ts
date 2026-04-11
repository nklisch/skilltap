import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { installSkill } from "@skilltap/core";
import { createStandaloneSkillRepo, createTestEnv, runSkilltap, type TestEnv } from "@skilltap/test-utils";

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

describe("skilltap skills — empty state", () => {
  test("exits 0 and prints empty message", async () => {
    const { exitCode, stdout } = await runSkilltap(["skills"], homeDir, configDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No skills found");
  });
});

describe("skilltap skills — with installed skill", () => {
  test("shows installed skill name", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runSkilltap(["skills"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
      expect(stdout).toContain("Global");
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
      expect(parsed[0]?.name).toBe("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("--global shows only global skills", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runSkilltap(
        ["skills", "--global"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("--project exits 0", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode } = await runSkilltap(
        ["skills", "--project"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("aliases", () => {
  test("skilltap list routes to skills view", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      const { exitCode, stdout } = await runSkilltap(["list"], homeDir, configDir);
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });
});
