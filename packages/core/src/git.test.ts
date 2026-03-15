import { lstat } from "node:fs/promises";
import { afterEach, describe, expect, test } from "bun:test";
import { createStandaloneSkillRepo } from "@skilltap/test-utils";
import { $ } from "bun";
import { makeTmpDir, removeTmpDir } from "./fs";
import { checkGitInstalled, clone, diff, fetch, flipUrlProtocol, log, lsRemoteTags, pull, revParse } from "./git";

describe("checkGitInstalled", () => {
  test("returns ok when git is on PATH", async () => {
    const result = await checkGitInstalled();
    expect(result.ok).toBe(true);
  });
});

describe("clone", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;
  let dest: string | null = null;

  afterEach(async () => {
    if (dest) {
      await removeTmpDir(dest);
      dest = null;
    }
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("clones a local repo successfully", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    const result = await clone(repo.path, `${dest}/clone`);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.effectiveUrl).toBe(repo.path);

    const skillMd = Bun.file(`${dest}/clone/SKILL.md`);
    expect(await skillMd.exists()).toBe(true);
  });

  test("returns GitError for invalid URL and leaves no tmp dir behind", async () => {
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    const clonePath = `${dest}/clone`;
    const result = await clone(
      "https://invalid.invalid/no/such/repo.git",
      clonePath,
    );
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("git clone failed");
    // git does not create the destination directory on network failure
    expect(await lstat(clonePath).catch(() => null)).toBeNull();
  });
});

describe("revParse", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;
  let dest: string | null = null;

  afterEach(async () => {
    if (dest) {
      await removeTmpDir(dest);
      dest = null;
    }
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("returns a 40-char SHA after clone", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    const result = await revParse(`${dest}/clone`);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toMatch(/^[0-9a-f]{40}$/);
  });
});

describe("log", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;
  let dest: string | null = null;

  afterEach(async () => {
    if (dest) {
      await removeTmpDir(dest);
      dest = null;
    }
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("returns commit entries with sha, message, date", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    const result = await log(`${dest}/clone`, 5);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.length).toBeGreaterThan(0);
    // biome-ignore lint/style/noNonNullAssertion: asserted non-empty above
    const entry = result.value[0]!;
    expect(entry.sha).toMatch(/^[0-9a-f]{40}$/);
    expect(typeof entry.message).toBe("string");
    expect(typeof entry.date).toBe("string");
    expect(entry.date.length).toBeGreaterThan(0);
  });
});

describe("pull and fetch", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;
  let dest: string | null = null;

  afterEach(async () => {
    if (dest) {
      await removeTmpDir(dest);
      dest = null;
    }
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("pull succeeds on an already-cloned repo", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    const result = await pull(`${dest}/clone`);
    expect(result.ok).toBe(true);
  });

  test("fetch succeeds on an already-cloned repo", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    const result = await fetch(`${dest}/clone`);
    expect(result.ok).toBe(true);
  });

  test("fetch returns GitError when remote is unreachable", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    // Point remote at an unreachable URL
    await $`git -C ${dest}/clone remote set-url origin https://invalid.invalid/no-such.git`.quiet();

    const result = await fetch(`${dest}/clone`);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("git fetch failed");
  });
});

describe("lsRemoteTags", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;

  afterEach(async () => {
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("lists tags from a local repo", async () => {
    repo = await createStandaloneSkillRepo();
    // Create tags in the fixture repo
    await $`git -C ${repo.path} tag v1.0.0`.quiet();
    await $`git -C ${repo.path} tag v2.0.0`.quiet();
    await $`git -C ${repo.path} tag unrelated`.quiet();

    const result = await lsRemoteTags(repo.path, "v*");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("v1.0.0");
    expect(result.value).toContain("v2.0.0");
    expect(result.value).not.toContain("unrelated");
  });

  test("returns all tags when no pattern given", async () => {
    repo = await createStandaloneSkillRepo();
    await $`git -C ${repo.path} tag v1.0.0`.quiet();
    await $`git -C ${repo.path} tag release-1`.quiet();

    const result = await lsRemoteTags(repo.path);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("v1.0.0");
    expect(result.value).toContain("release-1");
  });

  test("returns empty array when no tags match pattern", async () => {
    repo = await createStandaloneSkillRepo();
    await $`git -C ${repo.path} tag unrelated`.quiet();

    const result = await lsRemoteTags(repo.path, "v*");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("returns GitError for unreachable URL", async () => {
    const result = await lsRemoteTags("https://invalid.invalid/no-repo.git");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("git ls-remote failed");
  });
});

describe("diff", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null;
  let dest: string | null = null;

  afterEach(async () => {
    if (dest) {
      await removeTmpDir(dest);
      dest = null;
    }
    if (repo) {
      await repo.cleanup();
      repo = null;
    }
  });

  test("returns empty string when comparing HEAD to itself", async () => {
    repo = await createStandaloneSkillRepo();
    const destResult = await makeTmpDir();
    expect(destResult.ok).toBe(true);
    if (!destResult.ok) return;
    dest = destResult.value;

    await clone(repo.path, `${dest}/clone`);
    const sha = await revParse(`${dest}/clone`);
    expect(sha.ok).toBe(true);
    if (!sha.ok) return;

    const result = await diff(`${dest}/clone`, sha.value, sha.value);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toBe("");
  });
});

describe("flipUrlProtocol", () => {
  test("converts HTTPS to SSH scp-style", () => {
    expect(flipUrlProtocol("https://github.com/owner/repo.git")).toBe(
      "git@github.com:owner/repo.git",
    );
  });

  test("converts HTTPS without .git suffix and adds .git", () => {
    expect(flipUrlProtocol("https://github.com/owner/repo")).toBe(
      "git@github.com:owner/repo.git",
    );
  });

  test("converts SSH scp-style to HTTPS", () => {
    expect(flipUrlProtocol("git@github.com:owner/repo.git")).toBe(
      "https://github.com/owner/repo.git",
    );
  });

  test("converts SSH URL (ssh://git@...) to HTTPS", () => {
    expect(flipUrlProtocol("ssh://git@github.com/owner/repo.git")).toBe(
      "https://github.com/owner/repo.git",
    );
  });

  test("handles GitLab nested group path", () => {
    expect(flipUrlProtocol("https://gitlab.com/group/sub/repo.git")).toBe(
      "git@gitlab.com:group/sub/repo.git",
    );
    expect(flipUrlProtocol("git@gitlab.com:group/sub/repo.git")).toBe(
      "https://gitlab.com/group/sub/repo.git",
    );
  });

  test("returns null for local paths", () => {
    expect(flipUrlProtocol("/local/path/repo")).toBeNull();
    expect(flipUrlProtocol("./relative/repo")).toBeNull();
  });

  test("returns null for npm: sources", () => {
    expect(flipUrlProtocol("npm:@scope/pkg")).toBeNull();
  });

  test("returns null for http:// URLs", () => {
    expect(flipUrlProtocol("http://example.com/repo.git")).toBeNull();
  });
});
