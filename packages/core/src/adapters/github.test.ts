import { describe, expect, test } from "bun:test";
import { githubAdapter } from "./github";

describe("githubAdapter.canHandle", () => {
  test("accepts github: prefixed shorthand", () => {
    expect(githubAdapter.canHandle("github:user/repo")).toBe(true);
  });

  test("accepts bare owner/repo", () => {
    expect(githubAdapter.canHandle("user/repo")).toBe(true);
  });

  test("accepts owner/repo@ref", () => {
    expect(githubAdapter.canHandle("user/repo@v1.0")).toBe(true);
  });

  test("rejects https:// URLs", () => {
    expect(githubAdapter.canHandle("https://github.com/user/repo.git")).toBe(
      false,
    );
  });

  test("rejects git@ URLs", () => {
    expect(githubAdapter.canHandle("git@github.com:user/repo.git")).toBe(false);
  });

  test("rejects local paths", () => {
    expect(githubAdapter.canHandle("./path")).toBe(false);
    expect(githubAdapter.canHandle("/abs/path")).toBe(false);
    expect(githubAdapter.canHandle("~/home/path")).toBe(false);
  });

  test("rejects bare names with no slash", () => {
    expect(githubAdapter.canHandle("somename")).toBe(false);
  });
});

describe("githubAdapter.resolve", () => {
  test("formats github:user/repo to GitHub URL", async () => {
    const result = await githubAdapter.resolve("github:user/repo");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe("https://github.com/user/repo.git");
      expect(result.value.adapter).toBe("github");
      expect(result.value.ref).toBeUndefined();
    }
  });

  test("formats bare owner/repo", async () => {
    const result = await githubAdapter.resolve("owner/myrepo");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe("https://github.com/owner/myrepo.git");
      expect(result.value.adapter).toBe("github");
    }
  });

  test("extracts @ref suffix", async () => {
    const result = await githubAdapter.resolve("user/repo@v1.0");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe("https://github.com/user/repo.git");
      expect(result.value.ref).toBe("v1.0");
      expect(result.value.adapter).toBe("github");
    }
  });

  test("extracts @ref from github: prefixed", async () => {
    const result = await githubAdapter.resolve("github:user/repo@main");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe("https://github.com/user/repo.git");
      expect(result.value.ref).toBe("main");
    }
  });

  test("errors on missing repo part", async () => {
    const result = await githubAdapter.resolve("github:user");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Invalid GitHub source");
      expect(result.error.hint).toContain("owner/repo");
    }
  });

  test("errors on too many path parts", async () => {
    const result = await githubAdapter.resolve("github:user/repo/extra");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Invalid GitHub source");
    }
  });
});
