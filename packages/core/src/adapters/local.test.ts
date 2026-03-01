import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { localAdapter } from "./local";

describe("localAdapter.canHandle", () => {
  test("accepts ./ paths", () => {
    expect(localAdapter.canHandle("./path/to/skill")).toBe(true);
  });

  test("accepts absolute / paths", () => {
    expect(localAdapter.canHandle("/abs/path")).toBe(true);
  });

  test("accepts ~/ paths", () => {
    expect(localAdapter.canHandle("~/path")).toBe(true);
  });

  test("rejects https:// URLs", () => {
    expect(localAdapter.canHandle("https://github.com/user/repo")).toBe(false);
  });

  test("rejects github: shorthand", () => {
    expect(localAdapter.canHandle("github:user/repo")).toBe(false);
  });

  test("rejects bare owner/repo", () => {
    expect(localAdapter.canHandle("user/repo")).toBe(false);
  });
});

describe("localAdapter.resolve", () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await makeTmpDir();
  });

  afterEach(async () => {
    await removeTmpDir(tmpDir);
  });

  test("resolves real temp directory successfully", async () => {
    const result = await localAdapter.resolve(tmpDir);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe(tmpDir);
      expect(result.value.adapter).toBe("local");
      expect(result.value.ref).toBeUndefined();
    }
  });

  test("expands ~/ correctly", async () => {
    // Use actual homedir to build a path that exists
    const home = homedir();
    const result = await localAdapter.resolve("~/");
    // homedir itself is a directory
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.url).toBe(resolve(home, ""));
    }
  });

  test("returns UserError for non-existent path", async () => {
    const result = await localAdapter.resolve(
      "/tmp/skilltap-nonexistent-xyz-12345",
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Path does not exist");
    }
  });

  test("returns UserError for a file (not directory)", async () => {
    // Create a file inside tmpDir
    const filePath = `${tmpDir}/file.txt`;
    await Bun.write(filePath, "hello");
    const result = await localAdapter.resolve(filePath);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Path is not a directory");
    }
  });
});
