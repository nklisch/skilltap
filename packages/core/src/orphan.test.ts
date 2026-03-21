import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { loadInstalled, saveInstalled } from "./config";
import { skillCacheDir, skillInstallDir } from "./paths";
import {
  findOrphanRecords,
  formatOrphanReason,
  purgeOrphanRecords,
} from "./orphan";
import type { InstalledJson, InstalledSkill } from "./schemas/installed";

type Env = { SKILLTAP_HOME?: string; XDG_CONFIG_HOME?: string };

let savedEnv: Env;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  savedEnv = {
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
  };
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  if (savedEnv.XDG_CONFIG_HOME === undefined)
    delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

const NOW = "2024-01-01T00:00:00.000Z";

function makeSkill(overrides: Partial<InstalledSkill>): InstalledSkill {
  return {
    name: "test-skill",
    description: "A test skill",
    repo: "https://github.com/example/repo",
    ref: null,
    sha: null,
    scope: "global",
    path: null,
    tap: null,
    also: [],
    installedAt: NOW,
    updatedAt: NOW,
    active: true,
    ...overrides,
  };
}

function makeInstalled(skills: InstalledSkill[]): InstalledJson {
  return { version: 1, skills };
}

describe("formatOrphanReason", () => {
  test("formats directory-missing", () => {
    expect(formatOrphanReason("directory-missing")).toBe(
      "install directory missing from disk",
    );
  });

  test("formats cache-missing", () => {
    expect(formatOrphanReason("cache-missing")).toBe(
      "git cache directory missing",
    );
  });

  test("formats cache-subdir-missing", () => {
    expect(formatOrphanReason("cache-subdir-missing")).toBe(
      "skill subdirectory removed from upstream repo",
    );
  });

  test("formats link-target-missing", () => {
    expect(formatOrphanReason("link-target-missing")).toBe(
      "symlink target no longer exists",
    );
  });
});

describe("findOrphanRecords", () => {
  test("returns empty for skill with existing directory", async () => {
    const skill = makeSkill({ name: "healthy-skill" });
    const installDir = skillInstallDir("healthy-skill", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects directory-missing for standalone skill", async () => {
    const skill = makeSkill({ name: "missing-skill" });
    // Do NOT create the directory
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.record.name).toBe("missing-skill");
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("detects directory-missing for disabled skill", async () => {
    const skill = makeSkill({ name: "disabled-skill", active: false });
    // Do NOT create the disabled dir
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("no orphan when disabled skill dir exists", async () => {
    const { skillDisabledDir } = await import("./paths");
    const skill = makeSkill({ name: "disabled-ok", active: false });
    const disabledDir = skillDisabledDir("disabled-ok", "global");
    await mkdir(disabledDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects cache-missing for multi-skill", async () => {
    const skill = makeSkill({
      name: "multi-skill",
      path: ".agents/skills/multi-skill",
    });
    // Do NOT create the cache dir
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("cache-missing");
  });

  test("detects cache-subdir-missing when cache exists but subdirectory doesn't", async () => {
    const repoUrl = "https://github.com/example/multi-repo";
    const subdir = ".agents/skills/multi-skill";
    const skill = makeSkill({
      name: "multi-skill",
      repo: repoUrl,
      path: subdir,
    });

    // Create cache dir with .git but NOT the subdir
    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    // Also create install dir so that's not the problem
    const installDir = skillInstallDir("multi-skill", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("cache-subdir-missing");
  });

  test("no orphan when cache and subdir both exist", async () => {
    const repoUrl = "https://github.com/example/multi-repo2";
    const subdir = ".agents/skills/healthy-multi";
    const skill = makeSkill({
      name: "healthy-multi",
      repo: repoUrl,
      path: subdir,
    });

    const cacheDir = skillCacheDir(repoUrl);
    await mkdir(join(cacheDir, ".git"), { recursive: true });
    await mkdir(join(cacheDir, subdir), { recursive: true });
    const installDir = skillInstallDir("healthy-multi", "global");
    await mkdir(installDir, { recursive: true });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects link-target-missing for linked skill", async () => {
    const skill = makeSkill({
      name: "linked-skill",
      scope: "linked",
      path: join(homeDir, "nonexistent-target"),
    });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("link-target-missing");
  });

  test("no orphan for linked skill whose target exists", async () => {
    const targetDir = join(homeDir, "existing-linked-skill");
    await mkdir(targetDir, { recursive: true });

    const skill = makeSkill({
      name: "linked-skill",
      scope: "linked",
      path: targetDir,
    });

    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(0);
  });

  test("detects directory-missing for npm skill", async () => {
    const skill = makeSkill({
      name: "npm-skill",
      repo: "npm:@example/npm-skill",
    });
    // Do NOT create install dir
    const installed = makeInstalled([skill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.reason).toBe("directory-missing");
  });

  test("handles mixed orphans and healthy records", async () => {
    const healthySkill = makeSkill({ name: "healthy" });
    const installDir = skillInstallDir("healthy", "global");
    await mkdir(installDir, { recursive: true });

    const orphanSkill = makeSkill({ name: "orphan" });
    // No dir created for orphan

    const installed = makeInstalled([healthySkill, orphanSkill]);
    const orphans = await findOrphanRecords(installed);
    expect(orphans).toHaveLength(1);
    expect(orphans[0]!.record.name).toBe("orphan");
  });
});

describe("purgeOrphanRecords", () => {
  test("removes specified orphan records from installed.json", async () => {
    const skill = makeSkill({ name: "orphan-skill" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const orphans = [{ record: skill, reason: "directory-missing" as const }];
    const result = await purgeOrphanRecords(orphans, installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("orphan-skill");
    expect(installed.skills).toHaveLength(0);
  });

  test("saves installed.json after purging", async () => {
    const skill = makeSkill({ name: "to-purge" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const orphans = [{ record: skill, reason: "directory-missing" as const }];
    await purgeOrphanRecords(orphans, installed);

    // Reload from disk and verify
    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(0);
  });

  test("returns names of purged records", async () => {
    const skill1 = makeSkill({ name: "skill-a" });
    const skill2 = makeSkill({ name: "skill-b" });
    const installed = makeInstalled([skill1, skill2]);
    await saveInstalled(installed);

    const orphans = [
      { record: skill1, reason: "directory-missing" as const },
      { record: skill2, reason: "cache-missing" as const },
    ];
    const result = await purgeOrphanRecords(orphans, installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("skill-a");
    expect(result.value).toContain("skill-b");
  });

  test("handles empty orphan list (no-op)", async () => {
    const skill = makeSkill({ name: "healthy" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const result = await purgeOrphanRecords([], installed);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
    expect(installed.skills).toHaveLength(1);
  });

  test("preserves healthy records when purging orphans", async () => {
    const healthySkill = makeSkill({ name: "healthy" });
    const orphanSkill = makeSkill({ name: "orphan" });
    const installed = makeInstalled([healthySkill, orphanSkill]);
    await saveInstalled(installed);

    const orphans = [{ record: orphanSkill, reason: "directory-missing" as const }];
    await purgeOrphanRecords(orphans, installed);

    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(1);
    expect(reloaded.value.skills[0]!.name).toBe("healthy");
  });
});
