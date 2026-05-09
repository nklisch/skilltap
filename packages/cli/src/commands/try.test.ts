/**
 * CLI subprocess tests for `skilltap try`.
 *
 * Spec: docs/SPEC.md §v2.0 Try Command (lines 3212-3224):
 *   "Read-only preview... NEVER writes to install paths or state."
 *
 * The core `tryPreview` is unit-tested in packages/core/src/try.test.ts; here
 * we cover the CLI surface — exit codes, --json shape, and the never-writes
 * invariant that makes `try` safe to run on untrusted sources.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { readdir } from "node:fs/promises";
import { join } from "node:path";
import {
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

let homeDir: string;
let configDir: string;
let skillRepo: { path: string; cleanup: () => Promise<void> };

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  skillRepo = await createStandaloneSkillRepo();
});

afterAll(async () => {
  await skillRepo.cleanup();
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

describe("try — happy path", () => {
  test("exits 0 and prints skill name + 'Nothing was installed' tail", async () => {
    // Spec L3220 + try.ts L139: human-readable output ends with the
    // "preview, nothing installed" disclaimer. This is the user-facing
    // signal that try is safe.
    const { exitCode, stdout } = await runSkilltap(
      ["try", "skill", skillRepo.path, "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
    expect(stdout).toContain("Nothing was installed");
  });

  test("exits 1 with a hint when source does not exist", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["try", "skill", "/this/path/does/not/exist", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    // Error surface should include some message — we don't pin the exact
    // wording (that's an impl detail) but assert there's an error.
    expect(stderr.length).toBeGreaterThan(0);
  });

  test("exits 1 when type positional is missing", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["try", skillRepo.path, "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr.length).toBeGreaterThan(0);
  });

  test("exits 1 when type is not skill/plugin/mcp", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["try", "bogus", skillRepo.path, "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("Invalid try type");
  });
});

describe("try --json", () => {
  test("emits a parseable object with source/resolved/skills/scanned", async () => {
    // Spec L3223: "--json — emit the report as JSON instead of human-
    // readable text." try.ts L51-74 shapes the JSON payload.
    const { exitCode, stdout } = await runSkilltap(
      ["try", "skill", skillRepo.path, "--json", "--skip-scan"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    const payload = JSON.parse(stdout) as {
      source: string;
      type: string;
      resolved: { url: string };
      skills: Array<{ name: string; description: string }>;
      warnings: unknown[];
      scanned: boolean;
    };
    expect(payload.source).toBe(skillRepo.path);
    expect(payload.type).toBe("skill");
    expect(payload.resolved.url).toBeDefined();
    expect(Array.isArray(payload.skills)).toBe(true);
    expect(payload.skills.some((s) => s.name === "standalone-skill")).toBe(
      true,
    );
    expect(payload.scanned).toBe(false);
  });
});

describe("try — never writes to state or install paths (Spec L3220 invariant)", () => {
  test("does not create state.json, .agents/skills/, or any home/config files", async () => {
    // The crucial safety property: try previews an untrusted source. If it
    // can write to disk, the security model collapses. This test asserts
    // that home/config dirs are byte-identical (no new entries) before vs.
    // after a try run.
    const cleanHome = await makeTmpDir();
    const cleanConfig = await makeTmpDir();
    try {
      const beforeHome = await readdir(cleanHome);
      const beforeConfig = await readdir(cleanConfig);
      expect(beforeHome).toEqual([]);
      expect(beforeConfig).toEqual([]);

      const { exitCode } = await runSkilltap(
        ["try", "skill", skillRepo.path, "--skip-scan"],
        cleanHome,
        cleanConfig,
      );
      expect(exitCode).toBe(0);

      const afterHome = await readdir(cleanHome);
      const afterConfig = await readdir(cleanConfig);

      // Both dirs must remain empty — try never writes anywhere durable.
      expect(afterHome).toEqual([]);
      expect(afterConfig).toEqual([]);

      // Specifically, the canonical state file must not exist.
      const stateExists = await Bun.file(
        join(cleanConfig, "skilltap", "state.json"),
      ).exists();
      expect(stateExists).toBe(false);
    } finally {
      await removeTmpDir(cleanHome);
      await removeTmpDir(cleanConfig);
    }
  });

  test("--json invocation also never writes", async () => {
    // Same invariant under the --json output path (try.ts splits on json
    // late, so the impl shouldn't differ; but a future refactor could
    // accidentally short-circuit the wrong way — this test catches that).
    const cleanHome = await makeTmpDir();
    const cleanConfig = await makeTmpDir();
    try {
      const { exitCode } = await runSkilltap(
        ["try", "skill", skillRepo.path, "--json", "--skip-scan"],
        cleanHome,
        cleanConfig,
      );
      expect(exitCode).toBe(0);
      const afterHome = await readdir(cleanHome);
      const afterConfig = await readdir(cleanConfig);
      expect(afterHome).toEqual([]);
      expect(afterConfig).toEqual([]);
    } finally {
      await removeTmpDir(cleanHome);
      await removeTmpDir(cleanConfig);
    }
  });
});
