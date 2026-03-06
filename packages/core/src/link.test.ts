import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, symlink } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { linkSkill } from "./link";

type Env = { XDG_CONFIG_HOME?: string; SKILLTAP_HOME?: string };

let savedEnv: Env;
let tmpDir: string;
let configDir: string;
let homeDir: string;

beforeEach(async () => {
  savedEnv = {
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
  };
  tmpDir = await makeTmpDir();
  configDir = join(tmpDir, "config");
  homeDir = join(tmpDir, "home");
  await mkdir(join(configDir, "skilltap"), { recursive: true });
  await mkdir(homeDir, { recursive: true });
  process.env.XDG_CONFIG_HOME = configDir;
  process.env.SKILLTAP_HOME = homeDir;
});

afterEach(async () => {
  if (savedEnv.XDG_CONFIG_HOME === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  await removeTmpDir(tmpDir);
});

const VALID_SKILL_MD = `---
name: test-skill
description: A skill for link tests
license: MIT
---

## Instructions

Do stuff.
`;

async function makeSkillDir(name = "test-skill"): Promise<string> {
  const dir = join(tmpDir, name);
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "SKILL.md"), VALID_SKILL_MD.replace("test-skill", name));
  return dir;
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

describe("linkSkill — no SKILL.md", () => {
  test("returns error when localPath has no SKILL.md", async () => {
    const dir = join(tmpDir, "empty-dir");
    await mkdir(dir, { recursive: true });
    const result = await linkSkill(dir, { scope: "global" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("does not contain SKILL.md");
  });
});

describe("linkSkill — conflict", () => {
  test("returns error when skill is already installed", async () => {
    const dir = await makeSkillDir();

    // Pre-populate installed.json with the same skill name
    await Bun.write(
      join(configDir, "skilltap", "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "test-skill",
            description: "existing",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: "/some/path",
            tap: null,
            also: [],
            installedAt: "2024-01-01T00:00:00.000Z",
            updatedAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      }),
    );

    const result = await linkSkill(dir, { scope: "global" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already installed");
  });
});

describe("linkSkill — symlink error", () => {
  test("returns error when symlink destination already exists as a file", async () => {
    const dir = await makeSkillDir();

    // Pre-create a file at the install destination
    const installPath = join(homeDir, ".agents", "skills", "test-skill");
    await mkdir(join(homeDir, ".agents", "skills"), { recursive: true });
    await Bun.write(installPath, "blocking file");

    const result = await linkSkill(dir, { scope: "global" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Failed to create symlink");
  });
});

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

describe("linkSkill — happy path", () => {
  test("returns installed record on success", async () => {
    const dir = await makeSkillDir();
    const result = await linkSkill(dir, { scope: "global" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.name).toBe("test-skill");
    expect(result.value.scope).toBe("linked");
    expect(result.value.repo).toBeNull();
    expect(result.value.sha).toBeNull();
  });

  test("saves record to installed.json", async () => {
    const dir = await makeSkillDir();
    await linkSkill(dir, { scope: "global" });

    const f = Bun.file(join(configDir, "skilltap", "installed.json"));
    expect(await f.exists()).toBe(true);
    const data = await f.json();
    expect(data.skills).toHaveLength(1);
    expect(data.skills[0].name).toBe("test-skill");
    expect(data.skills[0].scope).toBe("linked");
  });

  test("creates agent symlinks when also is specified", async () => {
    const dir = await makeSkillDir();
    const result = await linkSkill(dir, { scope: "global", also: ["claude-code"] });
    expect(result.ok).toBe(true);

    const agentLinkPath = join(homeDir, ".claude", "skills", "test-skill");
    const stat = await Bun.file(agentLinkPath).exists();
    // The symlink path's parent exists (mkdir'd by createAgentSymlinks)
    // We check the symlink was created by seeing if the dir is accessible
    const { lstat } = await import("node:fs/promises");
    const linkStat = await lstat(agentLinkPath).catch(() => null);
    expect(linkStat).not.toBeNull();
  });

  test("project scope uses projectRoot for install path", async () => {
    const dir = await makeSkillDir();
    const projectRoot = join(tmpDir, "my-project");
    await mkdir(projectRoot, { recursive: true });

    const result = await linkSkill(dir, { scope: "project", projectRoot });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.path).toContain(projectRoot);
  });
});
