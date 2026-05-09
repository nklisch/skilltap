/**
 * CLI subprocess tests for first-time global install (no git repo, no project).
 *
 * Covers:
 *   - Test 1: Outside any git repo, install defaults to global with no prompt.
 *             Verifies state.json records scope="global" and no manifest is created.
 *   - Test 2: status --json from the same non-git cwd lists the installed skill
 *             and reports scope="global".
 *
 * Design: docs/designs/completed/e2e-v2.md §"Journey: First-time global install"
 */
import {
  afterAll,
  beforeAll,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

setDefaultTimeout(60_000);

let homeDir: string;
let configDir: string;
/** A plain tmp dir with NO .git ancestor — smart-scope-default should pick global. */
let nonGitCwd: string;
let skillRepo: { path: string; cleanup: () => Promise<void> };

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  // makeTmpDir gives us a bare /private/tmp/... path — no initRepo, so no .git.
  nonGitCwd = await makeTmpDir();
  skillRepo = await createStandaloneSkillRepo();
});

afterAll(async () => {
  await skillRepo.cleanup();
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
  await removeTmpDir(nonGitCwd);
});

describe("global install — outside any git repo", () => {
  // ── Test 1: first-time global install ────────────────────────────────────────

  test("1. install defaults to global scope and writes state.json with scope=global", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["install", "skill", skillRepo.path, "--yes", "--skip-scan"],
      homeDir,
      configDir,
      nonGitCwd,
    );
    if (exitCode !== 0) {
      console.error("install stderr:", stderr);
    }
    expect(exitCode).toBe(0);

    // Skill directory must exist under homeDir (global install location)
    const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
    const skillDirExists = await Bun.file(join(skillDir, "SKILL.md")).exists();
    expect(skillDirExists).toBe(true);

    // state.json in configDir (global state)
    const statePath = join(configDir, "skilltap", "state.json");
    const stateText = await readFile(statePath, "utf8");
    const state = JSON.parse(stateText) as {
      version: number;
      skills: Array<{ name: string; scope: string }>;
    };
    expect(state.version).toBe(2);
    const entry = state.skills.find((s) => s.name === "standalone-skill");
    expect(entry).toBeDefined();
    expect(entry?.scope).toBe("global");

    // No manifest or lockfile created in the non-project cwd
    const manifestExists = await Bun.file(
      join(nonGitCwd, "skilltap.toml"),
    ).exists();
    expect(manifestExists).toBe(false);
    const lockfileExists = await Bun.file(
      join(nonGitCwd, "skilltap.lock"),
    ).exists();
    expect(lockfileExists).toBe(false);
  });

  // ── Test 2: status --json from the same non-git cwd ──────────────────────────

  test("2. status --json lists the installed skill and reports global scope", async () => {
    // Relies on state written by Test 1 — tests in this describe block are sequential.
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["status", "--json"],
      homeDir,
      configDir,
      nonGitCwd,
    );
    if (exitCode !== 0) {
      console.error("status stderr:", stderr);
    }
    expect(exitCode).toBe(0);

    // Output must be valid JSON
    const payload = JSON.parse(stdout) as {
      scope: string;
      skills: Array<{ name: string }>;
    };

    // scope field is emitted by reportToJson in packages/cli/src/commands/status.ts
    expect(payload.scope).toBe("global");

    // The installed skill must appear in the skills array
    const found = payload.skills.some((s) => s.name === "standalone-skill");
    expect(found).toBe(true);
  });
});
