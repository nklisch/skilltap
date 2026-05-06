import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadManifest, manifestExists, saveManifest } from "./index";
import { loadLockfile, lockfileExists, saveLockfile } from "./lockfile";
import type { Lockfile, ProjectManifest } from "./schemas";

let projectRoot: string;
beforeEach(async () => {
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-mfst-"));
});
afterEach(async () => {
  await rm(projectRoot, { recursive: true, force: true });
});

describe("manifest load/save", () => {
  test("manifestExists returns false when missing", async () => {
    expect(await manifestExists(projectRoot)).toBe(false);
  });

  test("loadManifest returns defaults when missing", async () => {
    const result = await loadManifest(projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills).toEqual({});
    expect(result.value.plugins).toEqual({});
    expect(result.value.taps).toEqual({});
    expect(result.value.targets.also).toEqual([]);
  });

  test("save then load round-trips a populated manifest", async () => {
    const manifest: ProjectManifest = {
      targets: { also: ["claude-code", "cursor"], scope: "project" },
      skills: {
        "github:n/commit-helper": "^1.0",
      },
      plugins: {
        "github:c/dev-toolkit": {
          ref: "v2.1",
          components: { "test-skipper": false },
        },
      },
      taps: {
        home: "https://gitea.example.com/n/t",
      },
    };
    const saveResult = await saveManifest(projectRoot, manifest);
    expect(saveResult.ok).toBe(true);
    expect(await manifestExists(projectRoot)).toBe(true);

    const loadResult = await loadManifest(projectRoot);
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.targets.also).toEqual(["claude-code", "cursor"]);
    expect(loadResult.value.skills["github:n/commit-helper"]).toBe("^1.0");
    expect(loadResult.value.plugins["github:c/dev-toolkit"]).toEqual({
      ref: "v2.1",
      components: { "test-skipper": false },
    });
    expect(loadResult.value.taps["home"]).toBe("https://gitea.example.com/n/t");
  });

  test("loadManifest fails on invalid TOML", async () => {
    await writeFile(
      join(projectRoot, "skilltap.toml"),
      `not = valid = toml = oops`,
    );
    const result = await loadManifest(projectRoot);
    expect(result.ok).toBe(false);
  });

  test("loadManifest fails on schema mismatch", async () => {
    await writeFile(
      join(projectRoot, "skilltap.toml"),
      `[targets]\nscope = "linked"\n`,
    );
    const result = await loadManifest(projectRoot);
    expect(result.ok).toBe(false);
  });
});

describe("lockfile load/save", () => {
  test("lockfileExists returns false when missing", async () => {
    expect(await lockfileExists(projectRoot)).toBe(false);
  });

  test("loadLockfile returns defaults when missing", async () => {
    const result = await loadLockfile(projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.version).toBe(1);
    expect(result.value.skill).toEqual([]);
    expect(result.value.plugin).toEqual([]);
  });

  test("save then load round-trips a populated lockfile", async () => {
    const lockfile: Lockfile = {
      version: 1,
      skill: [
        {
          source: "github:n/commit-helper",
          ref: "v1.2.0",
          sha: "abc123",
          range: "^1.0",
        },
      ],
      plugin: [
        {
          source: "github:c/dev-toolkit",
          ref: "main",
          range: "*",
        },
      ],
    };
    const saveResult = await saveLockfile(projectRoot, lockfile);
    expect(saveResult.ok).toBe(true);
    expect(await lockfileExists(projectRoot)).toBe(true);

    const loadResult = await loadLockfile(projectRoot);
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.skill).toHaveLength(1);
    expect(loadResult.value.plugin).toHaveLength(1);
    expect(loadResult.value.skill[0].sha).toBe("abc123");
    expect(loadResult.value.plugin[0].sha).toBeUndefined();
  });
});
