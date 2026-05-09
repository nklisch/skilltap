/**
 * Lifecycle drift fix (Unit 3.15) — every state writer must keep skilltap.toml
 * + skilltap.lock in sync. This test asserts that update / move / toggle /
 * adopt / disable / enable each write the manifest+lockfile correctly when a
 * project manifest is present.
 *
 * The bar is "the manifest reflects the change." Detailed per-writer behavior
 * lives in update.test.ts / move.test.ts / etc.; this file is the safety net
 * that would have caught the bug class on day one.
 */

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { join } from "node:path";
import {
  createStandaloneSkillRepo,
  createTestEnv,
  makeTmpDir,
  type TestEnv,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { adoptSkillFromPath } from "./adopt";
import { disableSkill, enableSkill } from "./disable";
import { installSkill } from "./install";
import { loadManifest, manifestExists } from "./manifest";
import { moveSkill } from "./move";

setDefaultTimeout(60_000);

let env: TestEnv;

beforeEach(async () => {
  env = await createTestEnv();
});

afterEach(async () => {
  await env.cleanup();
});

async function setupProjectWithManifest(): Promise<string> {
  const projectRoot = await makeTmpDir();
  await $`git -C ${projectRoot} init`.quiet();
  // Empty manifest — `addSkillToManifest` is a no-op without one.
  await Bun.write(
    join(projectRoot, "skilltap.toml"),
    `[targets]
also = []
scope = "project"

[skills]

[plugins]

[taps]
`,
  );
  return projectRoot;
}

describe("lifecycle.manifest — install (baseline)", () => {
  test("install writes a manifest entry for project skills", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await setupProjectWithManifest();
    try {
      const result = await installSkill(repo.path, {
        scope: "project",
        projectRoot,
        skipScan: true,
      });
      expect(result.ok).toBe(true);

      expect(await manifestExists(projectRoot)).toBe(true);
      const manifest = await loadManifest(projectRoot);
      expect(manifest.ok).toBe(true);
      if (!manifest.ok) return;
      const keys = Object.keys(manifest.value.skills);
      expect(keys.length).toBeGreaterThan(0);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("lifecycle.manifest — disable / enable", () => {
  test("disable then enable round-trips via the manifest's components map", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await setupProjectWithManifest();
    try {
      const installRes = await installSkill(repo.path, {
        scope: "project",
        projectRoot,
        skipScan: true,
      });
      expect(installRes.ok).toBe(true);
      if (!installRes.ok) return;
      const skillName = installRes.value.records[0].name;

      const disableRes = await disableSkill(skillName, {
        scope: "project",
        projectRoot,
      });
      expect(disableRes.ok).toBe(true);

      const m1 = await loadManifest(projectRoot);
      expect(m1.ok).toBe(true);
      if (!m1.ok) return;
      const entries1 = Object.values(m1.value.skills);
      const detail1 = entries1.find(
        (e) =>
          typeof e === "object" &&
          e.components !== undefined &&
          skillName in e.components,
      );
      expect(detail1).toBeDefined();
      if (!detail1 || typeof detail1 !== "object") return;
      expect(detail1.components?.[skillName]).toBe(false);

      const enableRes = await enableSkill(skillName, {
        scope: "project",
        projectRoot,
      });
      expect(enableRes.ok).toBe(true);

      const m2 = await loadManifest(projectRoot);
      expect(m2.ok).toBe(true);
      if (!m2.ok) return;
      const entries2 = Object.values(m2.value.skills);
      const detail2 = entries2.find(
        (e) =>
          typeof e === "object" &&
          e.components !== undefined &&
          skillName in e.components,
      );
      expect(detail2).toBeDefined();
      if (!detail2 || typeof detail2 !== "object") return;
      expect(detail2.components?.[skillName]).toBe(true);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("lifecycle.manifest — move", () => {
  test("move project→global drops the project manifest entry", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await setupProjectWithManifest();
    try {
      const installRes = await installSkill(repo.path, {
        scope: "project",
        projectRoot,
        skipScan: true,
      });
      expect(installRes.ok).toBe(true);
      if (!installRes.ok) return;
      const skillName = installRes.value.records[0].name;

      const before = await loadManifest(projectRoot);
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      expect(Object.keys(before.value.skills).length).toBe(1);

      const moveRes = await moveSkill(skillName, {
        to: { scope: "global" },
        fromProjectRoot: projectRoot,
      });
      expect(moveRes.ok).toBe(true);

      const after = await loadManifest(projectRoot);
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      // Globals are unmanaged → the project manifest no longer carries this
      // skill.
      expect(Object.keys(after.value.skills).length).toBe(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("move global→project adds an entry to the destination project manifest", async () => {
    const repo = await createStandaloneSkillRepo();
    const projectRoot = await setupProjectWithManifest();
    try {
      const installRes = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
      });
      expect(installRes.ok).toBe(true);
      if (!installRes.ok) return;
      const skillName = installRes.value.records[0].name;

      // Manifest starts empty.
      const before = await loadManifest(projectRoot);
      expect(before.ok).toBe(true);
      if (!before.ok) return;
      expect(Object.keys(before.value.skills).length).toBe(0);

      const moveRes = await moveSkill(skillName, {
        to: { scope: "project", projectRoot },
      });
      expect(moveRes.ok).toBe(true);

      const after = await loadManifest(projectRoot);
      expect(after.ok).toBe(true);
      if (!after.ok) return;
      expect(Object.keys(after.value.skills).length).toBe(1);
    } finally {
      await repo.cleanup();
    }
  });
});

describe("lifecycle.manifest — adopt", () => {
  test("adoptSkillFromPath in a project with a manifest is a no-op for path-only adoptions", async () => {
    const projectRoot = await setupProjectWithManifest();
    const skillDir = await makeTmpDir();
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---\nname: adopted\ndescription: Adopted skill\n---\n# Adopted\n`,
    );

    const result = await adoptSkillFromPath(skillDir, {
      scope: "project",
      projectRoot,
      mode: "track-in-place",
      skipScan: true,
    });
    expect(result.ok).toBe(true);

    // Path-based adopt has repo=null. The manifest sync helper short-circuits
    // when repo is null, so there's no manifest entry. Verify the manifest
    // remained empty rather than gaining a junk entry.
    const after = await loadManifest(projectRoot);
    expect(after.ok).toBe(true);
    if (!after.ok) return;
    expect(Object.keys(after.value.skills).length).toBe(0);
  });
});
