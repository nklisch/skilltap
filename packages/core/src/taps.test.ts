import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { getConfigDir, loadConfig } from "./config";
import { installSkill } from "./install";
import type { TapEntry } from "./taps";
import {
  BUILTIN_TAP,
  addTap,
  ensureBuiltinTap,
  initTap,
  isBuiltinTapCloned,
  loadTaps,
  parseGitHubTapShorthand,
  removeTap,
  searchTaps,
  updateTap,
} from "./taps";

type Env = {
  SKILLTAP_HOME?: string;
  XDG_CONFIG_HOME?: string;
};

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

// Helper: create a local tap git repo with given skills
async function createLocalTap(
  skills: Array<{
    name: string;
    description: string;
    repo: string;
    tags?: string[];
  }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name: "test-tap",
    description: "Integration test tap",
    skills: skills.map((s) => ({ tags: [], ...s })),
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

// Helper: create a minimal standalone skill git repo
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

// ─── Unit tests: parseGitHubTapShorthand ───────────────────────────────────

describe("parseGitHubTapShorthand", () => {
  test("parses owner/repo", () => {
    expect(parseGitHubTapShorthand("user/my-tap")).toEqual({
      name: "my-tap",
      url: "https://github.com/user/my-tap.git",
    });
  });

  test("parses github:owner/repo", () => {
    expect(parseGitHubTapShorthand("github:acme/skills")).toEqual({
      name: "skills",
      url: "https://github.com/acme/skills.git",
    });
  });

  test("strips @ref suffix", () => {
    expect(parseGitHubTapShorthand("user/tap@main")).toEqual({
      name: "tap",
      url: "https://github.com/user/tap.git",
    });
  });

  test("returns null for bare names", () => {
    expect(parseGitHubTapShorthand("my-tap")).toBeNull();
  });

  test("returns null for full URLs", () => {
    expect(
      parseGitHubTapShorthand("https://github.com/user/repo.git"),
    ).toBeNull();
  });

  test("returns null for npm: prefix", () => {
    expect(parseGitHubTapShorthand("npm:my-package")).toBeNull();
  });

  test("returns null for local paths", () => {
    expect(parseGitHubTapShorthand("./local")).toBeNull();
    expect(parseGitHubTapShorthand("/abs/path")).toBeNull();
    expect(parseGitHubTapShorthand("~/home")).toBeNull();
  });

  test("returns null for three-part paths", () => {
    expect(parseGitHubTapShorthand("a/b/c")).toBeNull();
  });

  test("returns null for git@ URLs", () => {
    expect(
      parseGitHubTapShorthand("git@github.com:user/repo.git"),
    ).toBeNull();
  });
});

// ─── Unit tests: searchTaps ────────────────────────────────────────────────

describe("searchTaps", () => {
  const skills: TapEntry[] = [
    {
      tapName: "home",
      skill: {
        name: "commit-helper",
        description: "Generates commit messages",
        repo: "https://example.com/a",
        tags: ["git", "productivity"],
      },
    },
    {
      tapName: "home",
      skill: {
        name: "code-review",
        description: "Thorough code review with security focus",
        repo: "https://example.com/b",
        tags: ["review", "security"],
      },
    },
    {
      tapName: "community",
      skill: {
        name: "git-workflow",
        description: "Git branching workflow guidance",
        repo: "https://example.com/c",
        tags: ["git"],
      },
    },
  ];

  test("empty query returns all skills", () => {
    expect(searchTaps(skills, "")).toHaveLength(3);
    expect(searchTaps(skills, "   ")).toHaveLength(3);
  });

  test("matches by name (case-insensitive)", () => {
    const results = searchTaps(skills, "commit");
    expect(results).toHaveLength(1);
    expect(results[0]?.skill.name).toBe("commit-helper");
  });

  test("matches by description", () => {
    const results = searchTaps(skills, "security");
    expect(results).toHaveLength(1);
    expect(results[0]?.skill.name).toBe("code-review");
  });

  test("matches by tag", () => {
    const results = searchTaps(skills, "git");
    expect(results).toHaveLength(2);
    const names = results.map((r) => r.skill.name);
    expect(names).toContain("commit-helper");
    expect(names).toContain("git-workflow");
  });

  test("no match returns empty array", () => {
    expect(searchTaps(skills, "zzznomatch")).toHaveLength(0);
  });

  test("case-insensitive match", () => {
    const results = searchTaps(skills, "REVIEW");
    expect(results).toHaveLength(1);
    expect(results[0]?.skill.name).toBe("code-review");
  });
});

// ─── Integration tests: addTap ────────────────────────────────────────────

describe("addTap", () => {
  test("clones tap and updates config", async () => {
    const tap = await createLocalTap([
      {
        name: "my-skill",
        description: "A test skill",
        repo: "https://example.com/my-skill",
      },
    ]);
    try {
      const result = await addTap("home", tap.path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.skillCount).toBe(1);

      const config = await loadConfig();
      expect(config.ok).toBe(true);
      if (!config.ok) return;
      expect(config.value.taps).toHaveLength(1);
      expect(config.value.taps[0]?.name).toBe("home");
      expect(config.value.taps[0]?.url).toBe(tap.path);
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if tap name already exists", async () => {
    const tap = await createLocalTap([
      {
        name: "skill-a",
        description: "Skill A",
        repo: "https://example.com/a",
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await addTap("home", tap.path);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("already exists");
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if tap.json is missing", async () => {
    const emptyDir = await makeTmpDir();
    // Need at least one commit for git clone to work
    await Bun.write(join(emptyDir, ".gitkeep"), "");
    await initRepo(emptyDir);
    await commitAll(emptyDir);
    try {
      const result = await addTap("bad", emptyDir);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("tap.json");
    } finally {
      await removeTmpDir(emptyDir);
    }
  });
});

// ─── Integration tests: removeTap ─────────────────────────────────────────

describe("removeTap", () => {
  test("removes tap from config and filesystem", async () => {
    const tap = await createLocalTap([
      {
        name: "skill-a",
        description: "Skill A",
        repo: "https://example.com/a",
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await removeTap("home");
      expect(result.ok).toBe(true);

      const config = await loadConfig();
      expect(config.ok).toBe(true);
      if (!config.ok) return;
      expect(config.value.taps).toHaveLength(0);
    } finally {
      await tap.cleanup();
    }
  });

  test("errors if tap not configured", async () => {
    const result = await removeTap("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not configured");
  });
});

// ─── Integration tests: loadTaps ──────────────────────────────────────────

describe("loadTaps", () => {
  test("returns empty array when no taps configured", async () => {
    const result = await loadTaps();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
  });

  test("returns skills from configured tap", async () => {
    const tap = await createLocalTap([
      {
        name: "skill-a",
        description: "Skill A",
        repo: "https://example.com/a",
        tags: ["test"],
      },
      {
        name: "skill-b",
        description: "Skill B",
        repo: "https://example.com/b",
        tags: [],
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(2);
      expect(result.value[0]?.tapName).toBe("home");
      expect(result.value[0]?.skill.name).toBe("skill-a");
      expect(result.value[1]?.skill.name).toBe("skill-b");
    } finally {
      await tap.cleanup();
    }
  });

  test("merges skills from multiple taps", async () => {
    const tap1 = await createLocalTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    const tap2 = await createLocalTap([
      { name: "skill-b", description: "B", repo: "https://example.com/b" },
    ]);
    try {
      await addTap("tap1", tap1.path);
      await addTap("tap2", tap2.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(2);
      const tapNames = result.value.map((e) => e.tapName);
      expect(tapNames).toContain("tap1");
      expect(tapNames).toContain("tap2");
    } finally {
      await tap1.cleanup();
      await tap2.cleanup();
    }
  });
});

// ─── Integration tests: updateTap ─────────────────────────────────────────

describe("updateTap", () => {
  test("errors if named tap not found", async () => {
    const result = await updateTap("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not configured");
  });

  test("pulls updates and returns skill counts", async () => {
    const tap = await createLocalTap([
      {
        name: "skill-a",
        description: "Skill A",
        repo: "https://example.com/a",
      },
    ]);
    try {
      await addTap("home", tap.path);

      // Update tap.json in the source repo and commit
      const updatedJson = JSON.stringify(
        {
          name: "test-tap",
          description: "Updated tap",
          skills: [
            {
              name: "skill-a",
              description: "Skill A",
              repo: "https://example.com/a",
              tags: [],
            },
            {
              name: "skill-b",
              description: "Skill B",
              repo: "https://example.com/b",
              tags: [],
            },
          ],
        },
        null,
        2,
      );
      await Bun.write(join(tap.path, "tap.json"), updatedJson);
      await commitAll(tap.path, "add skill-b");

      const result = await updateTap("home");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.updated.home).toBe(2);
    } finally {
      await tap.cleanup();
    }
  });
});

// ─── Helper: simulate a cloned builtin tap (without hitting network) ────────

async function createClonedBuiltinTap(
  skills: Array<{ name: string; description: string; repo: string }> = [],
): Promise<{ sourceDir: string; cleanup: () => Promise<void> }> {
  // Create a source git repo that acts as the "remote" for the builtin tap
  const sourceDir = await makeTmpDir();
  const tapJson = { name: BUILTIN_TAP.name, description: "Built-in tap", skills: skills.map((s) => ({ tags: [], ...s })) };
  await Bun.write(join(sourceDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(sourceDir);
  await commitAll(sourceDir);

  // Clone it to tapDir(BUILTIN_TAP.name) — same path ensureBuiltinTap would use
  const { $ } = await import("bun");
  const destDir = join(getConfigDir(), "taps", BUILTIN_TAP.name);
  await mkdir(join(getConfigDir(), "taps"), { recursive: true });
  await $`git clone --depth=1 ${sourceDir} ${destDir}`.quiet();

  return { sourceDir, cleanup: () => removeTmpDir(sourceDir) };
}

// ─── Unit tests: isBuiltinTapCloned / ensureBuiltinTap ───────────────────

describe("isBuiltinTapCloned", () => {
  test("returns false when builtin tap is not cloned", async () => {
    expect(await isBuiltinTapCloned()).toBe(false);
  });

  test("returns true when tap.json exists in builtin tap dir", async () => {
    const { cleanup } = await createClonedBuiltinTap();
    try {
      expect(await isBuiltinTapCloned()).toBe(true);
    } finally {
      await cleanup();
    }
  });
});

describe("ensureBuiltinTap", () => {
  test("returns ok(undefined) immediately when already cloned", async () => {
    const { cleanup } = await createClonedBuiltinTap();
    try {
      const result = await ensureBuiltinTap();
      expect(result.ok).toBe(true);
    } finally {
      await cleanup();
    }
  });
});

// ─── loadTapJson edge cases (via addTap / loadTaps) ─────────────────────────

describe("loadTapJson — invalid tap.json", () => {
  test("addTap errors when cloned tap.json contains invalid JSON", async () => {
    const tapDir = await makeTmpDir();
    await Bun.write(join(tapDir, "tap.json"), "{ not valid json !!!");
    await initRepo(tapDir);
    await commitAll(tapDir);
    try {
      const result = await addTap("bad-json-tap", tapDir);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("Invalid JSON");
    } finally {
      await removeTmpDir(tapDir);
    }
  });

  test("addTap errors when cloned tap.json has invalid schema", async () => {
    const tapDir = await makeTmpDir();
    await Bun.write(join(tapDir, "tap.json"), JSON.stringify({ unexpected: true }));
    await initRepo(tapDir);
    await commitAll(tapDir);
    try {
      const result = await addTap("bad-schema-tap", tapDir);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("Invalid tap.json");
    } finally {
      await removeTmpDir(tapDir);
    }
  });
});

// ─── addTap: builtin tap name ────────────────────────────────────────────────

describe("addTap — builtin tap name", () => {
  test("errors when adding a tap with the builtin tap name", async () => {
    const tap = await createLocalTap([{ name: "skill-a", description: "A", repo: "https://example.com/a" }]);
    try {
      const result = await addTap(BUILTIN_TAP.name, tap.path);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain(BUILTIN_TAP.name);
      expect(result.error.message).toContain("built-in tap");
    } finally {
      await tap.cleanup();
    }
  });
});

// ─── removeTap: builtin tap paths ────────────────────────────────────────────

describe("removeTap — builtin tap", () => {
  test("errors when builtin tap is already disabled", async () => {
    // Write config with builtin_tap = false
    await mkdir(join(configDir, "skilltap"), { recursive: true });
    await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
    const result = await removeTap(BUILTIN_TAP.name);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already disabled");
  });

  test("disables builtin tap and removes local dir when enabled", async () => {
    const { cleanup } = await createClonedBuiltinTap();
    try {
      const result = await removeTap(BUILTIN_TAP.name);
      expect(result.ok).toBe(true);

      const config = await loadConfig();
      expect(config.ok).toBe(true);
      if (!config.ok) return;
      expect(config.value.builtin_tap).toBe(false);
    } finally {
      await cleanup();
    }
  });
});

// ─── updateTap: builtin tap paths ────────────────────────────────────────────

describe("updateTap — builtin tap", () => {
  test("errors when updating disabled builtin tap by name", async () => {
    await mkdir(join(configDir, "skilltap"), { recursive: true });
    await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
    const result = await updateTap(BUILTIN_TAP.name);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not configured");
  });

  test("errors when builtin tap is not yet cloned", async () => {
    const result = await updateTap(BUILTIN_TAP.name);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not yet cloned");
  });

  test("pulls builtin tap when updating by name", async () => {
    const { sourceDir, cleanup } = await createClonedBuiltinTap([
      { name: "skill-a", description: "A", repo: "https://example.com/a" },
    ]);
    try {
      const result = await updateTap(BUILTIN_TAP.name);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.updated[BUILTIN_TAP.name]).toBe(1);
    } finally {
      await cleanup();
    }
  });

  test("includes builtin tap when updating all and it is cloned", async () => {
    const { cleanup } = await createClonedBuiltinTap([
      { name: "skill-x", description: "X", repo: "https://example.com/x" },
    ]);
    try {
      const result = await updateTap();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.updated[BUILTIN_TAP.name]).toBe(1);
    } finally {
      await cleanup();
    }
  });
});

// ─── loadTaps: builtin tap entries ───────────────────────────────────────────

describe("loadTaps — builtin tap", () => {
  test("includes builtin tap skills when cloned", async () => {
    const { cleanup } = await createClonedBuiltinTap([
      { name: "builtin-skill", description: "A builtin skill", repo: "https://example.com/builtin" },
    ]);
    try {
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const builtinEntry = result.value.find((e) => e.tapName === BUILTIN_TAP.name);
      expect(builtinEntry).toBeDefined();
      expect(builtinEntry?.skill.name).toBe("builtin-skill");
    } finally {
      await cleanup();
    }
  });
});

// ─── initTap ─────────────────────────────────────────────────────────────────

describe("initTap", () => {
  test("creates tap directory with tap.json and git repo", async () => {
    const workDir = await makeTmpDir();
    const origCwd = process.cwd();
    process.chdir(workDir);
    try {
      const result = await initTap("my-new-tap");
      expect(result.ok).toBe(true);

      const tapJsonFile = Bun.file(join(workDir, "my-new-tap", "tap.json"));
      expect(await tapJsonFile.exists()).toBe(true);
      const tapJson = await tapJsonFile.json();
      expect(tapJson.name).toBe("my-new-tap");
      expect(tapJson.skills).toEqual([]);

      // Verify it's a git repo
      expect(await Bun.file(join(workDir, "my-new-tap", ".git", "HEAD")).exists()).toBe(true);
    } finally {
      process.chdir(origCwd);
      await removeTmpDir(workDir);
    }
  });
});

// ─── Integration tests: installSkill via tap name ─────────────────────────

describe("installSkill via tap name", () => {
  test("resolves skill name from configured tap", async () => {
    const skillRepo = await createLocalSkillRepo("tap-resolved-skill");
    const tap = await createLocalTap([
      {
        name: "tap-resolved-skill",
        description: "Test skill",
        repo: skillRepo.path,
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await installSkill("tap-resolved-skill", {
        scope: "global",
        skipScan: true,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.records).toHaveLength(1);
      expect(result.value.records[0]?.name).toBe("tap-resolved-skill");
      expect(result.value.records[0]?.tap).toBe("home");
      expect(result.value.records[0]?.repo).toBe(skillRepo.path);
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });

  test("resolves name@ref and sets ref in record", async () => {
    const skillRepo = await createLocalSkillRepo("versioned-skill");
    // Create a tag in the skill repo to use as a stable ref
    const { $ } = await import("bun");
    await $`git -C ${skillRepo.path} tag v1.0`.quiet();

    const tap = await createLocalTap([
      {
        name: "versioned-skill",
        description: "Versioned test skill",
        repo: skillRepo.path,
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await installSkill("versioned-skill@v1.0", {
        scope: "global",
        skipScan: true,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.records[0]?.ref).toBe("v1.0");
      expect(result.value.records[0]?.tap).toBe("home");
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });

  test("errors if no taps configured", async () => {
    const result = await installSkill("unknown-skill", {
      scope: "global",
      skipScan: true,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("No taps configured");
  });

  test("errors if skill not found in taps", async () => {
    const tap = await createLocalTap([
      {
        name: "some-other-skill",
        description: "Different skill",
        repo: "https://example.com/x",
      },
    ]);
    try {
      await addTap("home", tap.path);
      const result = await installSkill("nonexistent-skill", {
        scope: "global",
        skipScan: true,
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("not found in any configured tap");
    } finally {
      await tap.cleanup();
    }
  });

  test("uses onSelectTap when multiple taps match", async () => {
    const skillRepo = await createLocalSkillRepo("shared-skill");
    const tap1 = await createLocalTap([
      {
        name: "shared-skill",
        description: "Shared skill in tap1",
        repo: skillRepo.path,
      },
    ]);
    const tap2 = await createLocalTap([
      {
        name: "shared-skill",
        description: "Shared skill in tap2",
        repo: skillRepo.path,
      },
    ]);
    try {
      await addTap("tap1", tap1.path);
      await addTap("tap2", tap2.path);

      let capturedMatches: TapEntry[] = [];
      const result = await installSkill("shared-skill", {
        scope: "global",
        skipScan: true,
        onSelectTap: async (matches) => {
          capturedMatches = matches;
          // biome-ignore lint/style/noNonNullAssertion: matches guaranteed non-empty here
          return matches[0]!;
        },
      });
      expect(capturedMatches).toHaveLength(2);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.records[0]?.tap).toBe("tap1");
    } finally {
      await skillRepo.cleanup();
      await tap1.cleanup();
      await tap2.cleanup();
    }
  });
});
