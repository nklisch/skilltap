import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isInGitRepo } from "./paths";

let workDir: string;

beforeEach(async () => {
  workDir = await mkdtemp(join(tmpdir(), "skilltap-paths-"));
});

afterEach(async () => {
  await rm(workDir, { recursive: true, force: true });
});

describe("isInGitRepo", () => {
  test("returns null when no .git ancestor exists", async () => {
    const result = await isInGitRepo(workDir);
    expect(result).toBeNull();
  });

  test("returns the workdir when .git is present at the root", async () => {
    await mkdir(join(workDir, ".git"));
    const result = await isInGitRepo(workDir);
    expect(result).toBe(workDir);
  });

  test("walks up to find an ancestor's .git", async () => {
    await mkdir(join(workDir, ".git"));
    const nested = join(workDir, "src", "deep", "nested");
    await mkdir(nested, { recursive: true });
    const result = await isInGitRepo(nested);
    expect(result).toBe(workDir);
  });

  test("treats .git as a file (worktree case)", async () => {
    await writeFile(join(workDir, ".git"), "gitdir: /elsewhere/.git/worktrees/foo");
    const result = await isInGitRepo(workDir);
    expect(result).toBe(workDir);
  });
});
