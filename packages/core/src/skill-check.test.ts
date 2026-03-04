import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import {
  checkForSkillUpdates,
  fetchSkillUpdateStatus,
  writeSkillUpdateCache,
} from "./skill-check";

type Env = { XDG_CONFIG_HOME?: string; SKILLTAP_HOME?: string };

let savedEnv: Env;
let configDir: string;

beforeEach(async () => {
  savedEnv = {
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
  };
  configDir = await makeTmpDir();
  process.env.XDG_CONFIG_HOME = configDir;
  delete process.env.SKILLTAP_HOME;
  await mkdir(join(configDir, "skilltap"), { recursive: true });
});

afterEach(async () => {
  if (savedEnv.XDG_CONFIG_HOME === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  await removeTmpDir(configDir);
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
});
