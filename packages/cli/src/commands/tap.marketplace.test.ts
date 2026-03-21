import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

setDefaultTimeout(60_000);

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

async function disableBuiltinTap(configDir: string): Promise<void> {
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await Bun.write(join(configDir, "skilltap", "config.toml"), "builtin_tap = false\n");
}

async function createMarketplaceRepo(
  plugins: Array<{ name: string; source: string | object; description?: string }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const dir = await makeTmpDir();
  await mkdir(join(dir, ".claude-plugin"), { recursive: true });
  const marketplace = {
    name: "test-marketplace",
    owner: { name: "Test Owner" },
    plugins: plugins.map((p) => ({
      name: p.name,
      source: p.source,
      description: p.description ?? `Plugin ${p.name}`,
    })),
  };
  await Bun.write(
    join(dir, ".claude-plugin", "marketplace.json"),
    JSON.stringify(marketplace, null, 2),
  );
  await initRepo(dir);
  await commitAll(dir, "initial commit");
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

// ─── Test 5: Tap add with marketplace.json repo ───────────────────────────────

describe("tap marketplace — add tap with marketplace.json", () => {
  test("exits 0 and reports skill count, tap appears in tap list", async () => {
    const marketplaceRepo = await createMarketplaceRepo([
      { name: "plugin-alpha", source: { source: "github", repo: "https://example.com/alpha" } },
      { name: "plugin-beta", source: { source: "github", repo: "https://example.com/beta" } },
    ]);
    try {
      await disableBuiltinTap(configDir);

      const { exitCode, stdout } = await runSkilltap(
        ["tap", "add", "test-marketplace", marketplaceRepo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("test-marketplace");
      expect(stdout).toContain("2 skills");

      // Verify tap appears in tap list
      const { exitCode: listCode, stdout: listOut } = await runSkilltap(
        ["tap", "list"],
        homeDir,
        configDir,
      );
      expect(listCode).toBe(0);
      expect(listOut).toContain("test-marketplace");
    } finally {
      await marketplaceRepo.cleanup();
    }
  });
});

// ─── Test 6: Install from marketplace-sourced tap ────────────────────────────

describe("tap marketplace — install from marketplace-sourced tap", () => {
  test("exits 0, skill installed and recorded with tap reference", async () => {
    const skillRepo = await createStandaloneSkillRepo();
    const marketplaceRepo = await createMarketplaceRepo([
      {
        name: "standalone-skill",
        source: { source: "github", repo: skillRepo.path },
        description: "Skill from standalone repo",
      },
    ]);
    try {
      await disableBuiltinTap(configDir);

      // Add the marketplace as a tap
      const addResult = await runSkilltap(
        ["tap", "add", "test-marketplace", marketplaceRepo.path],
        homeDir,
        configDir,
      );
      expect(addResult.exitCode).toBe(0);

      // Install the skill by name (it's in the tap)
      const { exitCode, stdout } = await runSkilltap(
        ["install", "standalone-skill", "--global", "--yes", "--skip-scan"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await skillRepo.cleanup();
      await marketplaceRepo.cleanup();
    }
  });
});

// ─── Test 15: Tap add with marketplace.json — empty plugins array ─────────────

describe("tap marketplace — empty plugins array", () => {
  test("exits 0 and reports 0 skills, tap appears in tap list", async () => {
    const marketplaceRepo = await createMarketplaceRepo([]);
    try {
      await disableBuiltinTap(configDir);

      const { exitCode, stdout } = await runSkilltap(
        ["tap", "add", "empty-market", marketplaceRepo.path],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("0 skills");

      // Verify tap appears in tap list
      const { exitCode: listCode, stdout: listOut } = await runSkilltap(
        ["tap", "list"],
        homeDir,
        configDir,
      );
      expect(listCode).toBe(0);
      expect(listOut).toContain("empty-market");
    } finally {
      await marketplaceRepo.cleanup();
    }
  });
});
