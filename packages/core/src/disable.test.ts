import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { loadInstalled, saveInstalled } from "./config";
import { disableSkill, enableSkill } from "./disable";
import { createAgentSymlinks } from "./symlink";

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
  if (savedEnv.XDG_CONFIG_HOME === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

const NOW = "2026-01-01T00:00:00.000Z";

async function seedGlobalSkill(name: string, also: string[] = []) {
  const skillDir = join(homeDir, ".agents", "skills", name);
  await mkdir(skillDir, { recursive: true });
  await Bun.write(join(skillDir, "SKILL.md"), `---\nname: ${name}\n---\n`);
  await saveInstalled({
    version: 1,
    skills: [
      {
        name,
        description: "",
        repo: "https://github.com/example/repo",
        ref: "main",
        sha: null,
        scope: "global",
        path: null,
        tap: null,
        also,
        installedAt: NOW,
        updatedAt: NOW,
        active: true,
      },
    ],
  });
  if (also.length > 0) {
    await createAgentSymlinks(name, skillDir, also, "global");
  }
  return skillDir;
}

async function seedLinkedSkill(name: string, targetPath: string, also: string[] = []) {
  await saveInstalled({
    version: 1,
    skills: [
      {
        name,
        description: "",
        repo: null,
        ref: null,
        sha: null,
        scope: "linked",
        path: targetPath,
        tap: null,
        also,
        installedAt: NOW,
        updatedAt: NOW,
        active: true,
      },
    ],
  });
  if (also.length > 0) {
    await createAgentSymlinks(name, targetPath, also, "global");
  }
}

describe("disableSkill", () => {
  test("disables a managed global skill — moves to .disabled/, sets active=false", async () => {
    const skillDir = await seedGlobalSkill("my-skill", ["claude-code"]);
    const disabledDir = join(homeDir, ".agents", "skills", ".disabled", "my-skill");
    const symlinkPath = join(homeDir, ".claude", "skills", "my-skill");

    expect(await lstat(skillDir).catch(() => null)).not.toBeNull();
    expect(await lstat(symlinkPath).catch(() => null)).not.toBeNull();

    const result = await disableSkill("my-skill");
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Files moved to .disabled/
    expect(await lstat(skillDir).catch(() => null)).toBeNull();
    expect(await lstat(disabledDir).then((s) => s.isDirectory())).toBe(true);

    // Symlink removed
    expect(await lstat(symlinkPath).catch(() => null)).toBeNull();

    // Record updated
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    const record = loaded.value.skills.find((s) => s.name === "my-skill");
    expect(record?.active).toBe(false);
  });

  test("disables a linked skill — only removes symlinks, no file move", async () => {
    const targetDir = await makeTmpDir();
    try {
      await seedLinkedSkill("linked-skill", targetDir, ["claude-code"]);
      const symlinkPath = join(homeDir, ".claude", "skills", "linked-skill");

      expect(await lstat(symlinkPath).catch(() => null)).not.toBeNull();

      const result = await disableSkill("linked-skill");
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Symlink removed
      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();
      // Target still exists (we don't own it)
      expect(await lstat(targetDir).then((s) => s.isDirectory())).toBe(true);

      const loaded = await loadInstalled();
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      const record = loaded.value.skills.find((s) => s.name === "linked-skill");
      expect(record?.active).toBe(false);
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("errors on already-disabled skill", async () => {
    await seedGlobalSkill("my-skill");
    await disableSkill("my-skill");

    const result = await disableSkill("my-skill");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already disabled");
  });

  test("errors on non-existent skill", async () => {
    const result = await disableSkill("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });
});

describe("enableSkill", () => {
  test("enables a disabled global skill — moves back from .disabled/, recreates symlinks, sets active=true", async () => {
    const skillDir = await seedGlobalSkill("my-skill", ["claude-code"]);
    const disabledDir = join(homeDir, ".agents", "skills", ".disabled", "my-skill");
    const symlinkPath = join(homeDir, ".claude", "skills", "my-skill");

    await disableSkill("my-skill");
    expect(await lstat(disabledDir).catch(() => null)).not.toBeNull();
    expect(await lstat(symlinkPath).catch(() => null)).toBeNull();

    const result = await enableSkill("my-skill");
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Files moved back
    expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);
    expect(await lstat(disabledDir).catch(() => null)).toBeNull();

    // Symlink recreated
    expect(await lstat(symlinkPath).catch(() => null)).not.toBeNull();

    // Record updated
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    const record = loaded.value.skills.find((s) => s.name === "my-skill");
    expect(record?.active).toBe(true);
  });

  test("enables a linked skill — recreates symlinks from record.path", async () => {
    const targetDir = await makeTmpDir();
    try {
      await seedLinkedSkill("linked-skill", targetDir, ["claude-code"]);
      const symlinkPath = join(homeDir, ".claude", "skills", "linked-skill");

      await disableSkill("linked-skill");
      expect(await lstat(symlinkPath).catch(() => null)).toBeNull();

      const result = await enableSkill("linked-skill");
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Symlink recreated pointing to target
      expect(await lstat(symlinkPath).catch(() => null)).not.toBeNull();

      const loaded = await loadInstalled();
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      const record = loaded.value.skills.find((s) => s.name === "linked-skill");
      expect(record?.active).toBe(true);
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("errors on already-enabled skill", async () => {
    await seedGlobalSkill("my-skill");

    const result = await enableSkill("my-skill");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already enabled");
  });

  test("errors on non-existent skill", async () => {
    const result = await enableSkill("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
  });
});
