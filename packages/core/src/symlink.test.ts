import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat, readlink } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";

type Env = { SKILLTAP_HOME?: string };

let savedEnv: Env;
let homeDir: string;

beforeEach(async () => {
  savedEnv = { SKILLTAP_HOME: process.env.SKILLTAP_HOME };
  homeDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  await removeTmpDir(homeDir);
});

describe("createAgentSymlinks", () => {
  test("creates symlink at correct agent path for global scope", async () => {
    const targetDir = await makeTmpDir();
    try {
      const result = await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["claude-code"],
        "global",
      );
      expect(result.ok).toBe(true);
      const linkPath = join(homeDir, ".claude", "skills", "my-skill");
      const target = await readlink(linkPath);
      expect(target).toBe(targetDir);
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("creates symlink at correct agent path for project scope", async () => {
    const targetDir = await makeTmpDir();
    const projectDir = await makeTmpDir();
    try {
      const result = await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["cursor"],
        "project",
        projectDir,
      );
      expect(result.ok).toBe(true);
      const linkPath = join(projectDir, ".cursor", "skills", "my-skill");
      const target = await readlink(linkPath);
      expect(target).toBe(targetDir);
    } finally {
      await removeTmpDir(targetDir);
      await removeTmpDir(projectDir);
    }
  });

  test("creates parent directories if missing", async () => {
    const targetDir = await makeTmpDir();
    try {
      // homeDir has no .claude directory initially
      const parentDir = join(homeDir, ".claude", "skills");
      expect(await Bun.file(parentDir).exists()).toBe(false);

      const result = await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["claude-code"],
        "global",
      );
      expect(result.ok).toBe(true);

      const stat = await lstat(parentDir);
      expect(stat.isDirectory()).toBe(true);
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("returns UserError for unknown agent identifier", async () => {
    const targetDir = await makeTmpDir();
    try {
      const result = await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["unknown-agent"],
        "global",
      );
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("Unknown agent identifier");
      expect(result.error.message).toContain("unknown-agent");
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("returns UserError when symlink already exists at path", async () => {
    const targetDir = await makeTmpDir();
    const anotherTarget = await makeTmpDir();
    try {
      await createAgentSymlinks("my-skill", targetDir, ["claude-code"], "global");
      // Second call with same agent — symlink already exists
      const result = await createAgentSymlinks(
        "my-skill",
        anotherTarget,
        ["claude-code"],
        "global",
      );
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("Failed to create symlink");
    } finally {
      await removeTmpDir(targetDir);
      await removeTmpDir(anotherTarget);
    }
  });

  test("creates symlinks for multiple agents in one call", async () => {
    const targetDir = await makeTmpDir();
    try {
      const result = await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["claude-code", "cursor"],
        "global",
      );
      expect(result.ok).toBe(true);

      const claudePath = join(homeDir, ".claude", "skills", "my-skill");
      const cursorPath = join(homeDir, ".cursor", "skills", "my-skill");
      expect(await readlink(claudePath)).toBe(targetDir);
      expect(await readlink(cursorPath)).toBe(targetDir);
    } finally {
      await removeTmpDir(targetDir);
    }
  });
});

describe("removeAgentSymlinks", () => {
  test("succeeds with dangling symlink (target deleted, link exists)", async () => {
    const targetDir = await makeTmpDir();
    await createAgentSymlinks("my-skill", targetDir, ["claude-code"], "global");
    // Delete the target — the symlink becomes dangling
    await removeTmpDir(targetDir);

    const claudePath = join(homeDir, ".claude", "skills", "my-skill");
    // lstat on the symlink itself (not the target) should succeed — dangling link still has an inode
    expect(await lstat(claudePath).catch(() => null)).not.toBeNull();

    const result = await removeAgentSymlinks("my-skill", ["claude-code"], "global");
    expect(result.ok).toBe(true);
    // Link inode is gone
    expect(await lstat(claudePath).catch(() => null)).toBeNull();
  });

  test("removes existing symlinks and silently skips missing ones", async () => {
    const targetDir = await makeTmpDir();
    try {
      await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["claude-code", "cursor"],
        "global",
      );
      const claudePath = join(homeDir, ".claude", "skills", "my-skill");
      const cursorPath = join(homeDir, ".cursor", "skills", "my-skill");

      // Remove claude-code and gemini (gemini was never created — should not error)
      const result = await removeAgentSymlinks(
        "my-skill",
        ["claude-code", "gemini"],
        "global",
      );
      expect(result.ok).toBe(true);

      // claude-code link removed
      expect(await lstat(claudePath).catch(() => null)).toBeNull();
      // cursor link still exists
      expect(await lstat(cursorPath).catch(() => null)).not.toBeNull();
    } finally {
      await removeTmpDir(targetDir);
    }
  });

  test("treats linked scope as global for path resolution", async () => {
    const targetDir = await makeTmpDir();
    try {
      // Create symlink in global scope
      await createAgentSymlinks(
        "my-skill",
        targetDir,
        ["claude-code"],
        "global",
      );
      const claudePath = join(homeDir, ".claude", "skills", "my-skill");
      expect(await lstat(claudePath).catch(() => null)).not.toBeNull();

      // Remove with "linked" scope — should resolve to global and find the link
      const result = await removeAgentSymlinks(
        "my-skill",
        ["claude-code"],
        "linked",
      );
      expect(result.ok).toBe(true);
      expect(await lstat(claudePath).catch(() => null)).toBeNull();
    } finally {
      await removeTmpDir(targetDir);
    }
  });
});
