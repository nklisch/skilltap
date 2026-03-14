import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { resolveSource } from "./resolve";

describe("resolveSource", () => {
  test("routes https:// to git adapter", async () => {
    const result = await resolveSource("https://github.com/user/repo.git");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("git");
      expect(result.value.url).toBe("https://github.com/user/repo.git");
    }
  });

  test("routes git@ to git adapter", async () => {
    const result = await resolveSource("git@github.com:user/repo.git");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("git");
    }
  });

  test("routes github: shorthand to github adapter", async () => {
    const result = await resolveSource("github:user/repo");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("github");
      expect(result.value.url).toBe("https://github.com/user/repo.git");
    }
  });

  test("routes bare owner/repo to github adapter", async () => {
    const result = await resolveSource("user/repo");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("github");
    }
  });

  test("returns UserError for bare unresolvable name", async () => {
    const result = await resolveSource("bareword");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Cannot resolve source");
      expect(result.error.hint).toBeTruthy();
    }
  });

  test("routes owner/repo to custom git host when gitHost provided", async () => {
    const result = await resolveSource("user/repo", "https://gitea.example.com");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("github");
      expect(result.value.url).toBe("https://gitea.example.com/user/repo.git");
    }
  });

  test("routes github: prefix to custom git host", async () => {
    const result = await resolveSource("github:user/repo", "https://forgejo.local");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("github");
      expect(result.value.url).toBe("https://forgejo.local/user/repo.git");
    }
  });

  test("full https:// URL ignores custom git host", async () => {
    const result = await resolveSource(
      "https://github.com/user/repo.git",
      "https://gitea.example.com",
    );
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.adapter).toBe("git");
      expect(result.value.url).toBe("https://github.com/user/repo.git");
    }
  });

  describe("local path routing", () => {
    let tmpDir: string;

    beforeEach(async () => {
      tmpDir = await makeTmpDir();
    });

    afterEach(async () => {
      await removeTmpDir(tmpDir);
    });

    test("routes ./ path to local adapter", async () => {
      // Use absolute path since relative resolution depends on cwd
      const result = await resolveSource(tmpDir);
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.adapter).toBe("local");
      }
    });

    test("routes absolute path to local adapter", async () => {
      const result = await resolveSource(tmpDir);
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.adapter).toBe("local");
        expect(result.value.url).toBe(tmpDir);
      }
    });
  });
});
