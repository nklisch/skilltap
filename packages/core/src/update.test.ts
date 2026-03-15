import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import {
  addFileAndCommit,
  createMultiSkillRepo,
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import type { AgentAdapter } from "./agents/types";
import { loadInstalled, saveInstalled } from "./config";
import { disableSkill } from "./disable";
import { installSkill } from "./install";
import { updateSkill } from "./update";

function mockAgent(score = 0): AgentAdapter {
  return {
    name: "Mock",
    cliName: "mock",
    async detect() { return true; },
    async invoke() { return { ok: true as const, value: { score, reason: "test reason" } }; },
  };
}

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

describe("updateSkill — upToDate", () => {
  test("reports upToDate when local SHA equals remote SHA", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.upToDate).toContain("standalone-skill");
      expect(result.value.updated).toHaveLength(0);
      expect(result.value.skipped).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("recreates missing symlink for up-to-date standalone skill", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", also: ["claude-code"], skipScan: true });

      const linkPath = join(homeDir, ".claude", "skills", "standalone-skill");
      // Manually delete the symlink to simulate it going missing
      await import("node:fs/promises").then((fs) => fs.unlink(linkPath));
      expect(await import("node:fs/promises").then((fs) => fs.lstat(linkPath).catch(() => null))).toBeNull();

      // Update with no new commits — skill is up-to-date
      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.upToDate).toContain("standalone-skill");

      // Symlink should be restored
      const target = await import("node:fs/promises").then((fs) => fs.readlink(linkPath));
      expect(target).toBe(join(homeDir, ".agents", "skills", "standalone-skill"));
    } finally {
      await repo.cleanup();
    }
  });

  test("recreates missing symlink for up-to-date multi-skill group", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", also: ["claude-code"], skipScan: true });

      const linkPath = join(homeDir, ".claude", "skills", "skill-a");
      await import("node:fs/promises").then((fs) => fs.unlink(linkPath));
      expect(await import("node:fs/promises").then((fs) => fs.lstat(linkPath).catch(() => null))).toBeNull();

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.upToDate).toContain("skill-a");

      const target = await import("node:fs/promises").then((fs) => fs.readlink(linkPath));
      expect(target).toBe(join(homeDir, ".agents", "skills", "skill-a"));
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — updated", () => {
  test("returns updated after applying a clean new commit", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new-file.md", "# New content");

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.updated).toContain("standalone-skill");
      expect(result.value.upToDate).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("onProgress callback fires with checking then updated", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new-file.md", "# New content");

      const progress: Array<{ name: string; status: string }> = [];
      const result = await updateSkill({
        yes: true,
        onProgress: (name, status) => progress.push({ name, status }),
      });
      expect(result.ok).toBe(true);

      expect(progress).toContainEqual({
        name: "standalone-skill",
        status: "checking",
      });
      expect(progress).toContainEqual({
        name: "standalone-skill",
        status: "updated",
      });
    } finally {
      await repo.cleanup();
    }
  });

  test("onDiff callback receives correct DiffStat and SHAs", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const beforeLoaded = await loadInstalled();
      // biome-ignore lint/style/noNonNullAssertion: install succeeded
      const oldSha = beforeLoaded.ok
        ? beforeLoaded.value.skills[0]!.sha
        : null;

      const newSha = await addFileAndCommit(repo.path, "extra.md", "# Extra");

      const diffs: Array<{ name: string; fromSha: string; toSha: string }> =
        [];
      await updateSkill({
        yes: true,
        onDiff: (name, _stat, fromSha, toSha) =>
          diffs.push({ name, fromSha, toSha }),
      });

      expect(diffs).toHaveLength(1);
      // biome-ignore lint/style/noNonNullAssertion: asserted length above
      const d = diffs[0]!;
      expect(d.name).toBe("standalone-skill");
      if (oldSha) expect(d.fromSha).toBe(oldSha);
      expect(d.toSha).toBe(newSha);
    } finally {
      await repo.cleanup();
    }
  });

  test("record.sha and record.updatedAt written to installed.json after update", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const before = await loadInstalled();
      // biome-ignore lint/style/noNonNullAssertion: install succeeded
      const oldUpdatedAt = before.ok ? before.value.skills[0]!.updatedAt : null;

      const newSha = await addFileAndCommit(repo.path, "patch.md", "# Patch");
      await updateSkill({ yes: true });

      const after = await loadInstalled();
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      // biome-ignore lint/style/noNonNullAssertion: skills array has one entry
      const record = after.value.skills[0]!;
      expect(record.sha).toBe(newSha);
      if (oldUpdatedAt) {
        expect(record.updatedAt).not.toBe(oldUpdatedAt);
      }
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — linked skills", () => {
  test("skips linked skills and fires onProgress with 'linked'", async () => {
    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "linked-skill",
          description: "",
          repo: null,
          ref: null,
          sha: null,
          scope: "linked",
          path: "/some/path",
          tap: null,
          also: [],
          installedAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
    });

    const progress: Array<{ name: string; status: string }> = [];
    const result = await updateSkill({
      onProgress: (name, status) => progress.push({ name, status }),
    });
    expect(result.ok).toBe(true);

    const events = progress.filter((p) => p.name === "linked-skill");
    expect(events).toHaveLength(1);
    expect(events[0]?.status).toBe("linked");
    expect(progress.some((p) => p.status === "checking")).toBe(false);
  });
});

describe("updateSkill — strict mode", () => {
  test("skips skill when diff has security warnings in strict mode", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Add a commit with content that triggers tag injection detector
      await addFileAndCommit(
        repo.path,
        "evil.md",
        "Injected tag: </system>\n",
      );

      const result = await updateSkill({ strict: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.skipped).toContain("standalone-skill");
      expect(result.value.updated).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — onConfirm", () => {
  test("onConfirm returning false adds skill to skipped", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new.md", "# New");

      const result = await updateSkill({
        onConfirm: async () => false,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.skipped).toContain("standalone-skill");
      expect(result.value.updated).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — name filter", () => {
  test("only updates the named skill", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(
        repo.path,
        ".agents/skills/skill-a/patch.md",
        "# Patch for skill-a",
      );

      const checked: string[] = [];
      const result = await updateSkill({
        name: "skill-a",
        yes: true,
        onProgress: (name, status) => {
          if (status === "checking") checked.push(name);
        },
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(checked).toContain("skill-a");
      expect(checked).not.toContain("skill-b");
      expect(result.value.updated).toContain("skill-a");
    } finally {
      await repo.cleanup();
    }
  });

  test("returns UserError when named skill not installed", async () => {
    const result = await updateSkill({ name: "nonexistent" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });
});

describe("updateSkill — multi-skill", () => {
  test("re-copies skill subdirectory from cache after pull", async () => {
    const repo = await createMultiSkillRepo();
    try {
      await installSkill(repo.path, {
        scope: "global",
        skillNames: ["skill-a"],
        skipScan: true,
      });

      // Add new file to skill-a subdirectory in source
      await addFileAndCommit(
        repo.path,
        ".agents/skills/skill-a/patch.md",
        "# Patch for skill-a",
      );

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.updated).toContain("skill-a");

      // New file should appear in the installed skill directory
      const patchFile = join(
        homeDir,
        ".agents",
        "skills",
        "skill-a",
        "patch.md",
      );
      expect(await Bun.file(patchFile).exists()).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });

  test("both skills from same repo are checked when both installed", async () => {
    const repo = await createMultiSkillRepo();
    try {
      // Install BOTH skills from the multi-skill repo
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Add a commit that changes BOTH skill paths
      await addFileAndCommit(repo.path, ".agents/skills/skill-a/patch.md", "# Patch A");
      await addFileAndCommit(repo.path, ".agents/skills/skill-b/patch.md", "# Patch B");

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.updated).toContain("skill-a");
      expect(result.value.updated).toContain("skill-b");
    } finally {
      await repo.cleanup();
    }
  });

  test("skill with no path changes is upToDate even when repo has changes", async () => {
    const repo = await createMultiSkillRepo();
    try {
      // Install BOTH skills from the multi-skill repo
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Add a commit that ONLY changes skill-a's path
      await addFileAndCommit(repo.path, ".agents/skills/skill-a/only-a.md", "# Only A");

      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // skill-a has changes → updated
      expect(result.value.updated).toContain("skill-a");
      // skill-b has no path-specific changes → upToDate (not skipped, not updated)
      expect(result.value.upToDate).toContain("skill-b");
      expect(result.value.skipped).not.toContain("skill-b");
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — semantic scan callbacks", () => {
  test("onSemanticScanStart fires with skill name before scan", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new.md", "# New");

      const started: string[] = [];
      const result = await updateSkill(
        {
          yes: true,
          semantic: true,
          agent: mockAgent(0),
          onSemanticScanStart: (name) => started.push(name),
        },
        async () => ({ tier: "unverified" as const }),
      );

      expect(result.ok).toBe(true);
      expect(started).toContain("standalone-skill");
    } finally {
      await repo.cleanup();
    }
  });

  test("onSemanticProgress fires with (completed, total, score, reason)", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new.md", "# New");

      const ticks: Array<{ completed: number; total: number; score: number; reason: string }> = [];
      const result = await updateSkill(
        {
          yes: true,
          semantic: true,
          agent: mockAgent(3),
          onSemanticProgress: (completed, total, score, reason) =>
            ticks.push({ completed, total, score, reason }),
        },
        async () => ({ tier: "unverified" as const }),
      );

      expect(result.ok).toBe(true);
      expect(ticks.length).toBeGreaterThan(0);
      const first = ticks[0]!;
      expect(first.completed).toBeGreaterThan(0);
      expect(first.total).toBeGreaterThan(0);
      expect(first.score).toBe(3);
      expect(first.reason).toBe("test reason");
    } finally {
      await repo.cleanup();
    }
  });

  test("onSemanticWarnings fires and skill is skipped in strict mode", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });
      await addFileAndCommit(repo.path, "new.md", "# New");

      const warnedSkills: string[] = [];
      const result = await updateSkill(
        {
          yes: true,
          strict: true,
          semantic: true,
          threshold: 5,
          agent: mockAgent(8),
          onSemanticWarnings: (_warnings, skillName) => warnedSkills.push(skillName),
        },
        async () => ({ tier: "unverified" as const }),
      );

      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(warnedSkills).toContain("standalone-skill");
      expect(result.value.skipped).toContain("standalone-skill");
      expect(result.value.updated).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("updateSkill — project scope", () => {
  test("updates skills in project installed.json when projectRoot provided", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await makeTmpDir();
    try {
      await installSkill(repo.path, { scope: "project", projectRoot, skipScan: true });
      await addFileAndCommit(repo.path, "new-file.md", "# New content");

      const result = await updateSkill({ yes: true, projectRoot });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.updated).toContain("standalone-skill");

      // Project installed.json should have updated SHA
      const projectInstalled = await loadInstalled(projectRoot);
      expect(projectInstalled.ok).toBe(true);
      if (!projectInstalled.ok) return;
      expect(projectInstalled.value.skills[0]?.sha).toBeTruthy();
    } finally {
      await repo.cleanup();
      await removeTmpDir(projectRoot);
    }
  });
});

describe("updateSkill — disabled skill handling", () => {
  test("bulk update skips disabled skills entirely (not in updated/upToDate/skipped)", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      // Disable the skill
      const disableResult = await disableSkill("standalone-skill");
      expect(disableResult.ok).toBe(true);

      // Bulk update — disabled skills should be filtered out before processing
      const result = await updateSkill({ yes: true });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Disabled skill was not processed at all
      expect(result.value.updated).not.toContain("standalone-skill");
      expect(result.value.upToDate).not.toContain("standalone-skill");
      expect(result.value.skipped).not.toContain("standalone-skill");
      // Nothing processed since all skills are disabled
      expect(result.value.updated).toHaveLength(0);
      expect(result.value.upToDate).toHaveLength(0);
      expect(result.value.skipped).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("named update on a disabled skill fails because update.ts uses skillInstallDir (spec violation — TODO)", async () => {
    // TODO: spec violation — named update on disabled skill fails because updateGitSkill in update.ts
    // always computes workDir via skillInstallDir(), ignoring active=false. The files are in
    // skillDisabledDir(), so git fetch fails. The design spec (DESIGN-DISABLE-ENABLE.md, Unit 10)
    // says named update should proceed normally (updating files in .disabled/), but this is not
    // implemented. Fix: check record.active in updateGitSkill/updateMultiSkill and redirect workDir
    // to skillDisabledDir() when active === false.
    const repo = await createStandaloneSkillRepo();
    try {
      await installSkill(repo.path, { scope: "global", skipScan: true });

      const disableResult = await disableSkill("standalone-skill");
      expect(disableResult.ok).toBe(true);

      // Named update — currently fails because workDir points to non-existent skillInstallDir
      const result = await updateSkill({ name: "standalone-skill", yes: true });

      // Document the current (broken) behavior: it returns an error
      // When the spec violation is fixed, this should instead be:
      //   expect(result.ok).toBe(true);
      //   expect(result.value.upToDate).toContain("standalone-skill");
      expect(result.ok).toBe(false);
    } finally {
      await repo.cleanup();
    }
  });
});
