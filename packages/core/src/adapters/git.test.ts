import { describe, expect, test } from "bun:test";
import { gitAdapter } from "./git";

describe("gitAdapter.canHandle", () => {
  test("accepts https:// URLs", () => {
    expect(gitAdapter.canHandle("https://github.com/user/repo.git")).toBe(true);
  });

  test("accepts http:// URLs", () => {
    expect(gitAdapter.canHandle("http://example.com/repo.git")).toBe(true);
  });

  test("accepts git@ URLs", () => {
    expect(gitAdapter.canHandle("git@github.com:user/repo.git")).toBe(true);
  });

  test("accepts ssh:// URLs", () => {
    expect(gitAdapter.canHandle("ssh://git@github.com/user/repo.git")).toBe(
      true,
    );
  });

  test("rejects github: shorthand", () => {
    expect(gitAdapter.canHandle("github:user/repo")).toBe(false);
  });

  test("rejects bare owner/repo", () => {
    expect(gitAdapter.canHandle("user/repo")).toBe(false);
  });

  test("rejects local paths", () => {
    expect(gitAdapter.canHandle("./path/to/skill")).toBe(false);
    expect(gitAdapter.canHandle("/abs/path")).toBe(false);
    expect(gitAdapter.canHandle("~/home/path")).toBe(false);
  });
});

describe("gitAdapter.resolve", () => {
  test("returns url unchanged with adapter 'git'", async () => {
    const url = "https://github.com/user/repo.git";
    const result = await gitAdapter.resolve(url);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe(url);
      expect(result.value.adapter).toBe("git");
      expect(result.value.ref).toBeUndefined();
    }
  });

  test("passes through git@ URL unchanged", async () => {
    const url = "git@github.com:user/repo.git";
    const result = await gitAdapter.resolve(url);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe(url);
      expect(result.value.adapter).toBe("git");
    }
  });
});
