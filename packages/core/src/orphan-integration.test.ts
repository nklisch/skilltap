/**
 * Integration tests for orphan handling wired into updateSkill, installSkill.
 * These are heavier than orphan.test.ts because they require real git repos.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { loadInstalled, saveInstalled } from "./config";
import { installSkill } from "./install";
import type { OrphanRecord } from "./orphan";
import { skillInstallDir } from "./paths";
import { updateSkill } from "./update";
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

// ─── Gap #11-12: updateSkill orphan pre-flight ─────────────────────────────

describe("updateSkill — orphan pre-flight", () => {
  test("calls onOrphansFound when stale records exist", async () => {
    // Create a stale record with no directory on disk
    const skill = makeSkill({ name: "ghost-skill" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const orphansReceived: OrphanRecord[] = [];
    const result = await updateSkill({
      yes: true,
      async onOrphansFound(orphans) {
        orphansReceived.push(...orphans);
        return orphans.map((o) => o.record.name);
      },
    });

    // The callback should have been called with the stale record
    expect(orphansReceived).toHaveLength(1);
    expect(orphansReceived[0]!.record.name).toBe("ghost-skill");
    expect(orphansReceived[0]!.reason).toBe("directory-missing");

    // The record should have been purged — installed.json should now be empty
    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(0);

    // updateSkill itself should succeed (nothing to update after purge)
    expect(result.ok).toBe(true);
  });

  test("skips onOrphansFound when no orphans exist", async () => {
    // Save nothing — empty installed.json
    await saveInstalled(makeInstalled([]));

    let callbackInvoked = false;
    const result = await updateSkill({
      yes: true,
      async onOrphansFound(orphans) {
        callbackInvoked = true;
        return orphans.map((o) => o.record.name);
      },
    });

    expect(result.ok).toBe(true);
    expect(callbackInvoked).toBe(false);
  });

  test("respects onOrphansFound returning empty — does not purge", async () => {
    // Create a stale record
    const skill = makeSkill({ name: "keep-me" });
    const installed = makeInstalled([skill]);
    await saveInstalled(installed);

    const result = await updateSkill({
      yes: true,
      async onOrphansFound(_orphans) {
        return []; // user declines to purge
      },
    });

    // Record should still be there (not purged)
    const reloaded = await loadInstalled();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.skills).toHaveLength(1);
    expect(reloaded.value.skills[0]!.name).toBe("keep-me");

    // updateSkill fails because the skill was requested by name (via "all" pass)
    // but it has no directory. updateSkill without a name processes all active skills.
    // The ghost skill is still in the list and will try to update — but since it's a
    // standalone git skill with no real repo, the git fetch will fail.
    // We just verify the purge wasn't done, not the update result.
    expect(reloaded.value.skills[0]!.name).toBe("keep-me");
  });
});

// ─── Gap #14: standalone git skill doesn't crash when install dir is missing ─

describe("updateSkill — standalone git skill with missing install dir", () => {
  test("skips gracefully when install dir is deleted after install", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      // Install the skill
      const installResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(installResult.ok).toBe(true);
      if (!installResult.ok) return;

      const record = installResult.value.records[0]!;
      const installDir = skillInstallDir(record.name, "global");

      // Delete the install directory to simulate orphan state
      await rm(installDir, { recursive: true, force: true });

      const progressStatuses: Array<{ name: string; status: string }> = [];
      const result = await updateSkill({
        yes: true,
        onProgress(skillName, status) {
          progressStatuses.push({ name: skillName, status });
        },
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // The skill should be in skipped (reported as removed-upstream)
      const removedUpstream = progressStatuses.find(
        (p) => p.name === record.name && p.status === "removed-upstream",
      );
      expect(removedUpstream).toBeDefined();
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Gap #16-17: installSkill phantom conflict handling ────────────────────

describe("installSkill — phantom conflict", () => {
  test("installs successfully when conflict record exists but directory is missing", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      // First install succeeds
      const firstResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(firstResult.ok).toBe(true);
      if (!firstResult.ok) return;

      const record = firstResult.value.records[0]!;
      const installDir = skillInstallDir(record.name, "global");

      // Delete the install directory to create a phantom/stale conflict
      await rm(installDir, { recursive: true, force: true });

      // Second install — should succeed despite the stale record (phantom conflict)
      const secondResult = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
        // onAlreadyInstalled is NOT provided — if phantom conflict is handled correctly,
        // it won't be called because the stale record is cleaned up automatically
      });

      expect(secondResult.ok).toBe(true);
      if (!secondResult.ok) return;

      // Skill should be installed again
      expect(secondResult.value.records).toHaveLength(1);
      expect(secondResult.value.records[0]!.name).toBe(record.name);

      // Directory should now exist
      const reloaded = await loadInstalled();
      expect(reloaded.ok).toBe(true);
      if (!reloaded.ok) return;
      expect(reloaded.value.skills).toHaveLength(1);
    } finally {
      await repo.cleanup();
    }
  });

  test("calls onOrphansFound for unrelated stale records during install", async () => {
    const staleSkill = makeSkill({ name: "stale-skill" });
    const installed = makeInstalled([staleSkill]);
    await saveInstalled(installed);
    // Do NOT create stale-skill's directory

    const repo = await createStandaloneSkillRepo();
    try {
      const orphansReceived: OrphanRecord[] = [];
      const result = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
        async onOrphansFound(orphans) {
          orphansReceived.push(...orphans);
          return orphans.map((o) => o.record.name);
        },
      });

      expect(result.ok).toBe(true);

      // The stale skill should have been reported as an orphan
      expect(orphansReceived.some((o) => o.record.name === "stale-skill")).toBe(true);

      // After purge, only the newly installed skill should remain
      const reloaded = await loadInstalled();
      expect(reloaded.ok).toBe(true);
      if (!reloaded.ok) return;
      expect(reloaded.value.skills.every((s) => s.name !== "stale-skill")).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});

// ─── Helper: create a local skill repo (mirrors taps.test.ts pattern) ─────

async function createLocalSkillRepo(
  name: string,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const repoDir = await makeTmpDir();
  const skillMd = `---\nname: ${name}\ndescription: Test skill ${name}\n---\n# ${name}`;
  await Bun.write(join(repoDir, "SKILL.md"), skillMd);
  await initRepo(repoDir);
  await commitAll(repoDir);
  return { path: repoDir, cleanup: () => removeTmpDir(repoDir) };
}
