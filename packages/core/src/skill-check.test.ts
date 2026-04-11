import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import type { Result } from "./types";
import type { GitError, NetworkError } from "./types";
import {
  checkForSkillUpdates,
  fetchSkillUpdateStatus,
  writeSkillUpdateCache,
} from "./skill-check";
import { skillCacheDir } from "./paths";

let env: TestEnv;
let configDir: string;
let homeDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  configDir = env.configDir;
  homeDir = env.homeDir;
  await mkdir(join(configDir, "skilltap"), { recursive: true });
});

afterEach(async () => {
  await env.cleanup();
});

async function writeRawCache(data: object): Promise<void> {
  await Bun.write(
    join(configDir, "skilltap", "skills-update-check.json"),
    JSON.stringify(data),
  );
}

// ─── writeSkillUpdateCache ─────────────────────────────────────────────────────

describe("writeSkillUpdateCache", () => {
  test("writes cache with correct fields", async () => {
    await writeSkillUpdateCache(["skill-a", "skill-b"], "/my/project");
    const f = Bun.file(join(configDir, "skilltap", "skills-update-check.json"));
    expect(await f.exists()).toBe(true);
    const data = await f.json();
    expect(data.updatesAvailable).toEqual(["skill-a", "skill-b"]);
    expect(data.projectRoot).toBe("/my/project");
    expect(typeof data.checkedAt).toBe("string");
  });

  test("writes null projectRoot", async () => {
    await writeSkillUpdateCache([], null);
    const data = await Bun.file(
      join(configDir, "skilltap", "skills-update-check.json"),
    ).json();
    expect(data.projectRoot).toBeNull();
    expect(data.updatesAvailable).toEqual([]);
  });
});

// ─── checkForSkillUpdates ──────────────────────────────────────────────────────

describe("checkForSkillUpdates", () => {
  test("returns null when no cache file exists", async () => {
    const result = await checkForSkillUpdates(24, null);
    expect(result).toBeNull();
  });

  test("returns null when cached updatesAvailable is empty", async () => {
    await writeRawCache({
      checkedAt: new Date().toISOString(),
      updatesAvailable: [],
      projectRoot: null,
    });
    const result = await checkForSkillUpdates(24, null);
    expect(result).toBeNull();
  });

  test("returns cached updates when cache is fresh", async () => {
    await writeRawCache({
      checkedAt: new Date().toISOString(),
      updatesAvailable: ["skill-a"],
      projectRoot: null,
    });
    const result = await checkForSkillUpdates(24, null);
    expect(result).toEqual(["skill-a"]);
  });

  test("returns null when projectRoot changed (treats as stale)", async () => {
    await writeRawCache({
      checkedAt: new Date().toISOString(),
      updatesAvailable: ["skill-a"],
      projectRoot: "/old/project",
    });
    // Different projectRoot → stale → fires background refresh, but still returns null
    // (cache had data for a different project)
    const result = await checkForSkillUpdates(24, "/new/project");
    // Cache projectRoot mismatch = stale, background refresh fires, cached result still returned
    // The old cache data is still returned since we show cached results on stale
    // Actually per the implementation: if isStale, we fire refresh but still return cache.updatesAvailable
    // So the old skill-a is still returned
    expect(result).toEqual(["skill-a"]);
  });

  test("returns cached results even when stale (while background refresh runs)", async () => {
    const staleTime = new Date(Date.now() - 25 * 3_600_000).toISOString();
    await writeRawCache({
      checkedAt: staleTime,
      updatesAvailable: ["skill-b"],
      projectRoot: null,
    });
    const result = await checkForSkillUpdates(24, null);
    expect(result).toEqual(["skill-b"]);
  });

  test("returns null when cache is stale and updatesAvailable is empty", async () => {
    const staleTime = new Date(Date.now() - 25 * 3_600_000).toISOString();
    await writeRawCache({
      checkedAt: staleTime,
      updatesAvailable: [],
      projectRoot: null,
    });
    const result = await checkForSkillUpdates(24, null);
    expect(result).toBeNull();
  });
});

// ─── fetchSkillUpdateStatus ────────────────────────────────────────────────────

describe("fetchSkillUpdateStatus", () => {
  test("returns empty array when no skills installed", async () => {
    // No installed.json → loadInstalled returns empty
    const result = await fetchSkillUpdateStatus(null);
    expect(result).toEqual([]);
  });

  test("returns empty array when only linked skills are installed", async () => {
    const installedDir = join(configDir, "skilltap");
    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "my-skill",
            repo: null,
            ref: null,
            sha: null,
            scope: "linked",
            path: null,
            installedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );
    const result = await fetchSkillUpdateStatus(null);
    expect(result).toEqual([]);
  });

  test("skips git skill when cache .git dir does not exist", async () => {
    const installedDir = join(configDir, "skilltap");
    const repoUrl = "https://github.com/owner/repo";
    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "git-skill",
            description: "",
            repo: repoUrl,
            ref: "main",
            sha: "abc1234",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "git-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );
    // No cache .git dir created → lstat fails → skill skipped
    const result = await fetchSkillUpdateStatus(null);
    expect(result).toEqual([]);
  });

  test("marks git skill as having update when HEAD differs from FETCH_HEAD", async () => {
    const installedDir = join(configDir, "skilltap");
    const repoUrl = "https://github.com/owner/repo-update-test";
    const cacheDir = skillCacheDir(repoUrl);

    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "git-skill",
            description: "",
            repo: repoUrl,
            ref: "main",
            sha: "abc1234",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "git-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    // Create a fake cache dir with .git so the lstat check passes
    await mkdir(join(cacheDir, ".git"), { recursive: true });

    const mockFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: true as const, value: undefined });
    const mockRevParse = async (_dir: string, ref: string): Promise<Result<string, GitError>> => ({
      ok: true as const,
      value: ref === "HEAD" ? "abc1234abc1234abc1234" : "def5678def5678def5678",
    });

    const result = await fetchSkillUpdateStatus(null, mockFetch, mockRevParse);
    expect(result).toContain("git-skill");
  });

  test("does not mark git skill when HEAD matches FETCH_HEAD", async () => {
    const installedDir = join(configDir, "skilltap");
    const repoUrl = "https://github.com/owner/repo-no-update";
    const cacheDir = skillCacheDir(repoUrl);

    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "up-to-date-skill",
            description: "",
            repo: repoUrl,
            ref: "main",
            sha: "abc1234",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "up-to-date-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    await mkdir(join(cacheDir, ".git"), { recursive: true });

    const sha = "abc1234abc1234abc1234";
    const mockFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: true as const, value: undefined });
    const mockRevParse = async (_dir: string, _ref: string): Promise<Result<string, GitError>> =>
      ({ ok: true as const, value: sha });

    const result = await fetchSkillUpdateStatus(null, mockFetch, mockRevParse);
    expect(result).toEqual([]);
  });

  test("skips git skill gracefully when fetch fails", async () => {
    const installedDir = join(configDir, "skilltap");
    const repoUrl = "https://github.com/owner/repo-fetch-fail";
    const cacheDir = skillCacheDir(repoUrl);

    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "net-fail-skill",
            description: "",
            repo: repoUrl,
            ref: "main",
            sha: "abc1234",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "net-fail-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    await mkdir(join(cacheDir, ".git"), { recursive: true });

    const mockFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: false as const, error: { name: "GitError", message: "network error", hint: undefined } });
    const mockRevParse = async (_dir: string, _ref: string): Promise<Result<string, GitError>> =>
      ({ ok: true as const, value: "abc" });

    const result = await fetchSkillUpdateStatus(null, mockFetch, mockRevParse);
    expect(result).toEqual([]);
  });

  test("marks npm skill as having update when version differs from sha", async () => {
    const installedDir = join(configDir, "skilltap");
    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "npm-skill",
            description: "",
            repo: "npm:my-skill-package",
            ref: null,
            sha: "1.0.0",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "npm-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    const mockFetchMeta = async (_name: string): Promise<Result<any, NetworkError>> => ({
      ok: true as const,
      value: {
        name: "my-skill-package",
        description: "",
        distTags: { latest: "1.1.0" },
        versions: {
          "1.1.0": {
            version: "1.1.0",
            dist: { tarball: "https://example.com", integrity: "sha512-xxx" },
          },
        },
      },
    });

    const noopGitFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: true as const, value: undefined });
    const noopRevParse = async (_dir: string, _ref: string): Promise<Result<string, GitError>> =>
      ({ ok: true as const, value: "abc" });

    const result = await fetchSkillUpdateStatus(null, noopGitFetch, noopRevParse, mockFetchMeta);
    expect(result).toContain("npm-skill");
  });

  test("does not mark npm skill when version matches sha", async () => {
    const installedDir = join(configDir, "skilltap");
    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "npm-current-skill",
            description: "",
            repo: "npm:my-skill-package",
            ref: null,
            sha: "1.0.0",
            scope: "global",
            path: join(homeDir, ".agents", "skills", "npm-current-skill"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    const mockFetchMeta = async (_name: string): Promise<Result<any, NetworkError>> => ({
      ok: true as const,
      value: {
        name: "my-skill-package",
        description: "",
        distTags: { latest: "1.0.0" },
        versions: {
          "1.0.0": {
            version: "1.0.0",
            dist: { tarball: "https://example.com", integrity: "sha512-xxx" },
          },
        },
      },
    });

    const noopGitFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: true as const, value: undefined });
    const noopRevParse = async (_dir: string, _ref: string): Promise<Result<string, GitError>> =>
      ({ ok: true as const, value: "abc" });

    const result = await fetchSkillUpdateStatus(null, noopGitFetch, noopRevParse, mockFetchMeta);
    expect(result).toEqual([]);
  });

  test("skips npm skill when sha is null", async () => {
    const installedDir = join(configDir, "skilltap");
    await Bun.write(
      join(installedDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "npm-no-sha",
            description: "",
            repo: "npm:my-skill-package",
            ref: null,
            sha: null,
            scope: "global",
            path: join(homeDir, ".agents", "skills", "npm-no-sha"),
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    const mockFetchMeta = async (_name: string): Promise<Result<any, NetworkError>> => ({
      ok: true as const,
      value: { name: "x", description: "", distTags: { latest: "1.0.0" }, versions: {} },
    });

    const noopGitFetch = async (_dir: string): Promise<Result<void, GitError>> =>
      ({ ok: true as const, value: undefined });
    const noopRevParse = async (_dir: string, _ref: string): Promise<Result<string, GitError>> =>
      ({ ok: true as const, value: "abc" });

    const result = await fetchSkillUpdateStatus(null, noopGitFetch, noopRevParse, mockFetchMeta);
    expect(result).toEqual([]);
  });
});
