import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  type TestEnv,
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
  tapPluginToManifest,
  updateTap,
} from "./taps";
import type { TapPlugin } from "./schemas/tap";

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

  test("uses custom git host when provided", () => {
    expect(
      parseGitHubTapShorthand("user/my-tap", "https://gitea.example.com"),
    ).toEqual({
      name: "my-tap",
      url: "https://gitea.example.com/user/my-tap.git",
    });
  });

  test("strips trailing slash from custom git host", () => {
    expect(
      parseGitHubTapShorthand("user/repo", "https://gitea.example.com/"),
    ).toEqual({
      name: "repo",
      url: "https://gitea.example.com/user/repo.git",
    });
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

  test("attempts clone when builtin tap dir is missing (self-heal)", async () => {
    // No tap directory exists. updateTap should attempt a clone rather than
    // returning a UserError about "not yet cloned". The result is either ok
    // (network available) or a GitError — never a "not yet cloned" UserError.
    const result = await updateTap(BUILTIN_TAP.name);
    if (!result.ok) {
      expect(result.error.constructor.name).not.toBe("UserError");
    }
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

// ─── Helper: create a local marketplace git repo ───────────────────────────

async function createLocalMarketplace(
  plugins: Array<{ name: string; source: string; description?: string; category?: string }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const dir = await makeTmpDir();
  await mkdir(join(dir, ".claude-plugin"), { recursive: true });
  const marketplace = {
    name: "test-marketplace",
    owner: { name: "Test" },
    plugins: plugins.map((p) => ({
      name: p.name,
      source: p.source,
      ...(p.description ? { description: p.description } : {}),
      ...(p.category ? { category: p.category } : {}),
    })),
  };
  await Bun.write(
    join(dir, ".claude-plugin", "marketplace.json"),
    JSON.stringify(marketplace, null, 2),
  );
  await initRepo(dir);
  await commitAll(dir);
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

// ─── Integration tests: marketplace.json fallback ──────────────────────────

describe("loadTapJson — marketplace.json fallback", () => {
  test("addTap works with marketplace repo (no tap.json, has .claude-plugin/marketplace.json)", async () => {
    const mp = await createLocalMarketplace([
      {
        name: "mp-skill",
        source: "https://github.com/owner/mp-skill.git",
        description: "A marketplace skill",
      },
    ]);
    try {
      const result = await addTap("mp-tap", mp.path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.skillCount).toBe(1);
      expect(result.value.type).toBe("git");
    } finally {
      await mp.cleanup();
    }
  });

  test("tap.json takes precedence when both exist", async () => {
    const dir = await makeTmpDir();
    // Write tap.json with 2 skills
    const tapJson = {
      name: "precedence-tap",
      skills: [
        { name: "skill-a", description: "A", repo: "https://example.com/a", tags: [] },
        { name: "skill-b", description: "B", repo: "https://example.com/b", tags: [] },
      ],
    };
    await Bun.write(join(dir, "tap.json"), JSON.stringify(tapJson, null, 2));
    // Also write marketplace.json with 1 plugin
    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    const marketplace = {
      name: "test-marketplace",
      owner: { name: "Test" },
      plugins: [{ name: "mp-skill", source: "https://example.com/mp.git" }],
    };
    await Bun.write(
      join(dir, ".claude-plugin", "marketplace.json"),
      JSON.stringify(marketplace, null, 2),
    );
    await initRepo(dir);
    await commitAll(dir);
    try {
      const result = await addTap("precedence-tap", dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      // tap.json has 2 skills, marketplace.json has 1 — tap.json wins
      expect(result.value.skillCount).toBe(2);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns error mentioning both formats when neither exists", async () => {
    const emptyDir = await makeTmpDir();
    await Bun.write(join(emptyDir, ".gitkeep"), "");
    await initRepo(emptyDir);
    await commitAll(emptyDir);
    try {
      const result = await addTap("no-tap", emptyDir);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("tap.json");
      expect(result.error.message).toContain("marketplace.json");
    } finally {
      await removeTmpDir(emptyDir);
    }
  });
});

describe("loadTaps — marketplace taps", () => {
  test("marketplace tap skills appear in loadTaps() results", async () => {
    const mp = await createLocalMarketplace([
      {
        name: "mp-skill-a",
        source: "https://github.com/owner/mp-skill-a.git",
        description: "Marketplace skill A",
      },
      {
        name: "mp-skill-b",
        source: "https://github.com/owner/mp-skill-b.git",
        description: "Marketplace skill B",
      },
    ]);
    try {
      await addTap("mp-tap", mp.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const mpEntries = result.value.filter((e) => e.tapName === "mp-tap");
      expect(mpEntries).toHaveLength(2);
      expect(mpEntries[0]?.skill.name).toBe("mp-skill-a");
      expect(mpEntries[1]?.skill.name).toBe("mp-skill-b");
    } finally {
      await mp.cleanup();
    }
  });

  test("marketplace tap skills are searchable via searchTaps()", async () => {
    const mp = await createLocalMarketplace([
      {
        name: "pdf-extractor",
        source: "https://github.com/owner/pdf.git",
        description: "Extracts text from PDF files",
        category: "document",
      },
    ]);
    try {
      await addTap("mp-tap", mp.path);
      const loadResult = await loadTaps();
      expect(loadResult.ok).toBe(true);
      if (!loadResult.ok) return;

      const pdfResults = searchTaps(loadResult.value, "pdf");
      expect(pdfResults).toHaveLength(1);
      expect(pdfResults[0]?.skill.name).toBe("pdf-extractor");

      const docResults = searchTaps(loadResult.value, "document");
      expect(docResults).toHaveLength(1);
      expect(docResults[0]?.skill.name).toBe("pdf-extractor");
    } finally {
      await mp.cleanup();
    }
  });
});

// ─── Helper: create a local tap git repo with plugins ────────────────────────

async function createLocalTapWithPlugin(plugin: TapPlugin): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapPath = await makeTmpDir();
  const tapJson = {
    name: "test-tap",
    description: "Test tap with plugins",
    skills: [],
    plugins: [plugin],
  };
  await Bun.write(join(tapPath, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapPath);
  await commitAll(tapPath);
  return { path: tapPath, cleanup: () => removeTmpDir(tapPath) };
}

// ─── Unit tests: tapPluginToManifest ─────────────────────────────────────────

describe("tapPluginToManifest", () => {
  let tapPath: string;

  beforeEach(async () => {
    tapPath = await makeTmpDir();
  });

  afterEach(async () => {
    await removeTmpDir(tapPath);
  });

  test("converts skills with correct paths", async () => {
    const plugin: TapPlugin = {
      name: "my-plugin",
      description: "My plugin",
      skills: [{ name: "my-skill", path: "skills/my-skill", description: "A skill" }],
      agents: [],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const skillComp = result.value.components.find((c) => c.type === "skill");
    expect(skillComp).toBeDefined();
    expect(skillComp?.type === "skill" && skillComp.path).toBe("skills/my-skill");
  });

  test("sets format to skilltap and pluginRoot to tapDir", async () => {
    const plugin: TapPlugin = { name: "p", description: "", skills: [], agents: [], tags: [] };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.format).toBe("skilltap");
    expect(result.value.pluginRoot).toBe(tapPath);
  });

  test("returns empty components when no skills/mcp/agents", async () => {
    const plugin: TapPlugin = { name: "empty", description: "", skills: [], agents: [], tags: [] };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(0);
  });

  test("converts inline MCP servers", async () => {
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      mcpServers: { "my-db": { command: "npx", args: ["-y", "my-mcp"] } },
      agents: [],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const mcpComps = result.value.components.filter((c) => c.type === "mcp");
    expect(mcpComps).toHaveLength(1);
    const mcp = mcpComps[0];
    expect(mcp?.type === "mcp" && mcp.server.name).toBe("my-db");
  });

  test("converts file path MCP references", async () => {
    const mcpContent = JSON.stringify({ "file-db": { command: "node", args: ["server.js"] } });
    await Bun.write(join(tapPath, ".mcp.json"), mcpContent);
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      mcpServers: ".mcp.json",
      agents: [],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const mcpComps = result.value.components.filter((c) => c.type === "mcp");
    expect(mcpComps).toHaveLength(1);
    const mcp = mcpComps[0];
    expect(mcp?.type === "mcp" && mcp.server.name).toBe("file-db");
  });

  test("returns empty MCP list for missing MCP file (non-fatal)", async () => {
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      mcpServers: "nonexistent/.mcp.json",
      agents: [],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components.filter((c) => c.type === "mcp")).toHaveLength(0);
  });

  test("returns error for malformed MCP inline object", async () => {
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      mcpServers: { "bad-server": "not-an-object" } as Record<string, unknown>,
      agents: [],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(false);
  });

  test("reads agent frontmatter from .md files", async () => {
    const agentDir = join(tapPath, "agents");
    await mkdir(agentDir, { recursive: true });
    await Bun.write(
      join(agentDir, "my-agent.md"),
      "---\nname: my-agent\nmodel: sonnet\n---\nYou are helpful.\n",
    );
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      agents: [{ name: "my-agent", path: "agents/my-agent.md" }],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const agentComp = result.value.components.find((c) => c.type === "agent");
    expect(agentComp).toBeDefined();
    if (agentComp?.type !== "agent") return;
    expect(agentComp.frontmatter.model).toBe("sonnet");
  });

  test("includes agent with empty frontmatter when file missing", async () => {
    const plugin: TapPlugin = {
      name: "p",
      description: "",
      skills: [],
      agents: [{ name: "ghost", path: "agents/ghost.md" }],
      tags: [],
    };
    const result = await tapPluginToManifest(plugin, tapPath);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const agentComp = result.value.components.find((c) => c.type === "agent");
    expect(agentComp).toBeDefined();
    if (agentComp?.type !== "agent") return;
    expect(agentComp.frontmatter).toEqual({});
  });
});

// ─── Integration tests: loadTaps includes plugin entries ─────────────────────

describe("loadTaps — tap plugin entries", () => {
  test("includes plugin entries from tap plugins array", async () => {
    const plugin: TapPlugin = {
      name: "dev-toolkit",
      description: "Dev tools",
      skills: [],
      agents: [],
      tags: ["dev", "tools"],
    };
    const tap = await createLocalTapWithPlugin(plugin);
    try {
      await addTap("my-tap", tap.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const pluginEntries = result.value.filter((e) => e.tapPlugin !== undefined);
      expect(pluginEntries).toHaveLength(1);
      const entry = pluginEntries[0]!;
      expect(entry.tapName).toBe("my-tap");
      expect(entry.skill.name).toBe("dev-toolkit");
      expect(entry.skill.plugin).toBe(true);
      expect(entry.tapPlugin?.name).toBe("dev-toolkit");
    } finally {
      await tap.cleanup();
    }
  });

  test("skills-only taps still work unchanged", async () => {
    const tap = await createLocalTap([
      { name: "my-skill", description: "A skill", repo: "owner/repo" },
    ]);
    try {
      await addTap("skills-tap", tap.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const entries = result.value.filter((e) => e.tapName === "skills-tap");
      expect(entries).toHaveLength(1);
      expect(entries[0]?.tapPlugin).toBeUndefined();
    } finally {
      await tap.cleanup();
    }
  });

  test("searchTaps finds plugins by name and tags", async () => {
    const plugin: TapPlugin = {
      name: "dev-toolkit",
      description: "Developer tools collection",
      skills: [],
      agents: [],
      tags: ["dev", "tools"],
    };
    const tap = await createLocalTapWithPlugin(plugin);
    try {
      await addTap("tool-tap", tap.path);
      const loadResult = await loadTaps();
      expect(loadResult.ok).toBe(true);
      if (!loadResult.ok) return;
      const byName = searchTaps(loadResult.value, "dev-toolkit");
      expect(byName.length).toBeGreaterThanOrEqual(1);
      expect(byName[0]?.skill.name).toBe("dev-toolkit");
      const byTag = searchTaps(loadResult.value, "tools");
      expect(byTag.length).toBeGreaterThanOrEqual(1);
    } finally {
      await tap.cleanup();
    }
  });
});
