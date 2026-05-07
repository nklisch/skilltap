/**
 * CLI subprocess tests for `skilltap sync`.
 *
 * Spec: docs/SPEC.md §v2.0 Sync Command (lines 3144-3168).
 *
 * Behaviors covered here:
 *   - bare `sync` — read-only drift report; exit 0 even with drift; ends with
 *     "note: run skilltap sync --apply to execute this plan."
 *   - in-sync state — prints "✓ In sync. Manifest, lockfile, and state agree."
 *     and exits 0.
 *   - `sync --json` — emits a JSON object with `inSync` + `items` fields;
 *     no human-readable text.
 *   - `sync --apply` on in-sync state — prints "✓ In sync. Nothing to apply."
 *   - `sync --apply --strict` — exit code on failure.
 *
 * The `sync --apply` happy path is already covered in e2e-v2.test.ts; here we
 * focus on the read-only / json / strict behaviors that have no CLI coverage.
 *
 * Additional tests (drift workflow + lockfile/manifest adversarial cases):
 *   - Test 3: delete state.json out-of-band → drift report surfaces it
 *   - Test 4: sync --apply restores state from lockfile after state.json deletion
 *   - Test 5: manifest add (skill in manifest/lockfile but not state) → reinstalls on apply
 *   - Test 6: manifest remove (skill in state but not manifest) → uninstalls on apply
 *   - Test 7: --strict halts at first failure
 *   - A2: manifest TOML schema mismatch (non-bool components value) → Zod error
 *   - A3: lockfile version != 1 → Zod schema rejection
 *   - A18: lock-stale sha mismatch → drift report; apply skips it
 *   - A19: lock-orphan (lockfile only, no manifest, no state) → drift report; apply skips it
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

let homeDir: string;
let configDir: string;

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
});

afterAll(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

async function makeProjectRoot(opts: {
  manifestToml: string;
  lockfileToml?: string;
  stateJson?: string;
}): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const dir = await makeTmpDir();
  await initRepo(dir);
  await writeFile(join(dir, "skilltap.toml"), opts.manifestToml);
  if (opts.lockfileToml !== undefined) {
    await writeFile(join(dir, "skilltap.lock"), opts.lockfileToml);
  }
  if (opts.stateJson !== undefined) {
    await mkdir(join(dir, ".agents"), { recursive: true });
    await writeFile(join(dir, ".agents", "state.json"), opts.stateJson);
  }
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

const EMPTY_STATE = JSON.stringify({
  version: 2,
  skills: [],
  plugins: [],
});

describe("sync — read-only drift report (default mode)", () => {
  test("in-sync state prints '✓ In sync. Manifest, lockfile, and state agree.' and exits 0", async () => {
    // Spec §v2.0 Sync L3152: "If everything agrees, prints `✓ In sync.
    // Manifest, lockfile, and state agree.` and exits 0."
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("In sync");
      expect(stdout).toContain("Manifest, lockfile, and state agree");
    } finally {
      await proj.cleanup();
    }
  });

  test("drift exits 0 in read-only mode (no --apply) and emits the apply hint", async () => {
    // Spec §v2.0 Sync L3152: read-only inspection by default, "does not
    // auto-apply or prompt"; ends with "note: run skilltap sync --apply".
    const proj = await makeProjectRoot({
      // Manifest declares a skill that's not in state → 'add' drift.
      manifestToml:
        '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      // Read-only mode never exits non-zero just for drift — that's reserved
      // for --apply --strict failures.
      expect(exitCode).toBe(0);
      expect(stdout).toContain("drift report");
      expect(stdout).toContain("skilltap sync --apply");
    } finally {
      await proj.cleanup();
    }
  });

  test("drift report groups items by kind (e.g. '+ add')", async () => {
    // Spec §v2.0 Sync L3152: "drift report grouped by kind".
    const proj = await makeProjectRoot({
      manifestToml: '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(stdout).toContain("add");
      expect(stdout).toContain("github:example/missing");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync --json", () => {
  test("--json emits a parseable object with inSync + items", async () => {
    // Spec §v2.0 Sync L3157: "--json — output the plan as JSON instead".
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--json"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        inSync: boolean;
        items: unknown[];
      };
      expect(payload.inSync).toBe(true);
      expect(Array.isArray(payload.items)).toBe(true);
      expect(payload.items).toEqual([]);
    } finally {
      await proj.cleanup();
    }
  });

  test("--json on drift includes items array with kind/source/target", async () => {
    const proj = await makeProjectRoot({
      manifestToml: '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--json"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        inSync: boolean;
        items: Array<{ kind: string; source: string }>;
      };
      expect(payload.inSync).toBe(false);
      expect(payload.items.length).toBeGreaterThan(0);
      const addItem = payload.items.find((i) => i.kind === "add");
      expect(addItem).toBeDefined();
      expect(addItem?.source).toBe("github:example/missing");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync --apply on in-sync project", () => {
  test("prints '✓ In sync. Nothing to apply.' and exits 0", async () => {
    // Spec §v2.0 Sync L3152 + sync.ts L89: in-sync apply emits this exact
    // message and short-circuits before invoking install/remove.
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--apply"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("In sync");
      expect(stdout).toContain("Nothing to apply");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync — no project root", () => {
  test("exits 1 with an explanatory error when cwd has no .git and no skilltap.toml", async () => {
    // Spec §v2.0 Project Manifest L2880: "Presence of [skilltap.toml] is
    // what defines a 'skilltap project'." Sync needs a project root to
    // reconcile against — without one, there's nothing to do and the
    // misleading "trivially in-sync" no-op is worse than a clear error.
    const empty = await makeTmpDir();
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        empty,
      );
      expect(exitCode).toBe(1);
      expect(stderr.toLowerCase()).toContain("project root");
    } finally {
      await removeTmpDir(empty);
    }
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Bootstrap helper: install a real skill into a fresh project directory.
// Returns the project directory path. Caller owns cleanup via removeTmpDir.
//
// Uses its own isolated homeDir/configDir to avoid cross-test state pollution.
// ──────────────────────────────────────────────────────────────────────────────
async function bootstrapProjectWithSkill(
  skillPath: string,
): Promise<{ projDir: string; projHome: string; projConfig: string }> {
  const projDir = await makeTmpDir();
  const projHome = await makeTmpDir();
  const projConfig = await makeTmpDir();
  await initRepo(projDir);
  await writeFile(join(projDir, "skilltap.toml"), "");
  const { exitCode, stderr } = await runSkilltap(
    ["install", skillPath, "--project", "--yes", "--skip-scan"],
    projHome,
    projConfig,
    projDir,
  );
  if (exitCode !== 0) {
    throw new Error(`bootstrapProjectWithSkill: install failed: ${stderr}`);
  }
  return { projDir, projHome, projConfig };
}

// ──────────────────────────────────────────────────────────────────────────────
// Drift workflow tests (Tests 3–7)
// ──────────────────────────────────────────────────────────────────────────────

describe("sync — drift workflow: state.json deletion", () => {
  // Test 3 & 4 share a skill fixture created once for the describe block.
  let skillRepo: { path: string; cleanup: () => Promise<void> };

  beforeAll(async () => {
    skillRepo = await createStandaloneSkillRepo();
  });

  afterAll(async () => {
    await skillRepo.cleanup();
  });

  test("Test 3: delete state.json out-of-band → sync reports drift", async () => {
    // Setup: real install writes manifest + lockfile + state.json.
    // Then we delete state.json to simulate out-of-band mutation.
    const { projDir, projHome, projConfig } =
      await bootstrapProjectWithSkill(skillRepo.path);
    try {
      await rm(join(projDir, ".agents", "state.json"), { force: true });

      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        projHome,
        projConfig,
        projDir,
      );
      expect(exitCode).toBe(0);
      // Read-only sync emits the drift report header and the apply hint.
      expect(stdout).toContain("drift report");
      // The locked skill source should surface somewhere in the output
      // (it was in the lockfile, now absent from state → detected as drift).
      expect(stdout).toContain(skillRepo.path);
      expect(stdout).toContain("skilltap sync --apply");
    } finally {
      await removeTmpDir(projDir);
      await removeTmpDir(projHome);
      await removeTmpDir(projConfig);
    }
  });

  test("Test 4: sync --apply restores state from lockfile after state.json deletion", async () => {
    const { projDir, projHome, projConfig } =
      await bootstrapProjectWithSkill(skillRepo.path);
    try {
      await rm(join(projDir, ".agents", "state.json"), { force: true });

      const { exitCode } = await runSkilltap(
        ["sync", "--apply", "--skip-scan"],
        projHome,
        projConfig,
        projDir,
      );
      expect(exitCode).toBe(0);

      // state.json must be re-written with version 2 and the skill record back.
      const stateText = await readFile(
        join(projDir, ".agents", "state.json"),
        "utf8",
      );
      const state = JSON.parse(stateText) as {
        version: number;
        skills: Array<{ name: string }>;
      };
      expect(state.version).toBe(2);
      expect(state.skills.some((s) => s.name === "standalone-skill")).toBe(
        true,
      );

      // The on-disk install dir must also exist.
      const installDirExists = await Bun.file(
        join(projDir, ".agents", "skills", "standalone-skill", "SKILL.md"),
      ).exists();
      expect(installDirExists).toBe(true);
    } finally {
      await removeTmpDir(projDir);
      await removeTmpDir(projHome);
      await removeTmpDir(projConfig);
    }
  });
});

describe("sync — drift workflow: manifest add (skill in manifest/lock but not state)", () => {
  let skillRepo: { path: string; cleanup: () => Promise<void> };

  beforeAll(async () => {
    skillRepo = await createStandaloneSkillRepo();
  });

  afterAll(async () => {
    await skillRepo.cleanup();
  });

  test("Test 5a: --json reports inSync=false with an 'add' item", async () => {
    // Manifest declares the skill, lockfile has the entry, but state is empty.
    // This is the "fresh clone" shape.
    const proj = await makeProjectRoot({
      manifestToml: `[skills]\n"${skillRepo.path}" = "*"\n`,
      lockfileToml: `version = 1\n\n[[skill]]\nsource = "${skillRepo.path}"\nref = "main"\nrange = "*"\n`,
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--json"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        inSync: boolean;
        items: Array<{ kind: string; source: string }>;
      };
      expect(payload.inSync).toBe(false);
      const addItem = payload.items.find((i) => i.kind === "add");
      expect(addItem).toBeDefined();
      expect(addItem?.source).toBe(skillRepo.path);
    } finally {
      await proj.cleanup();
    }
  });

  test("Test 5b: sync --apply installs the skill declared in manifest", async () => {
    const proj = await makeProjectRoot({
      manifestToml: `[skills]\n"${skillRepo.path}" = "*"\n`,
      lockfileToml: `version = 1\n\n[[skill]]\nsource = "${skillRepo.path}"\nref = "main"\nrange = "*"\n`,
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode } = await runSkilltap(
        ["sync", "--apply", "--skip-scan"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);

      // state.json must now have the skill record.
      const stateText = await readFile(
        join(proj.path, ".agents", "state.json"),
        "utf8",
      );
      const state = JSON.parse(stateText) as {
        version: number;
        skills: Array<{ name: string }>;
      };
      expect(state.skills.some((s) => s.name === "standalone-skill")).toBe(
        true,
      );

      // Install dir on disk must exist.
      const skillMdExists = await Bun.file(
        join(proj.path, ".agents", "skills", "standalone-skill", "SKILL.md"),
      ).exists();
      expect(skillMdExists).toBe(true);
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync — drift workflow: manifest remove (skill in state but not manifest)", () => {
  let skillRepo: { path: string; cleanup: () => Promise<void> };

  beforeAll(async () => {
    skillRepo = await createStandaloneSkillRepo();
  });

  afterAll(async () => {
    await skillRepo.cleanup();
  });

  test("Test 6: sync --apply removes skill present in state but absent from manifest", async () => {
    // Bootstrap a real install so the skill is on disk + in state.json.
    const { projDir, projHome, projConfig } =
      await bootstrapProjectWithSkill(skillRepo.path);
    try {
      // Overwrite the manifest to declare NO skills — creating a "remove" drift.
      await writeFile(join(projDir, "skilltap.toml"), "");

      const { exitCode } = await runSkilltap(
        ["sync", "--apply", "--skip-scan"],
        projHome,
        projConfig,
        projDir,
      );
      expect(exitCode).toBe(0);

      // The install dir must be gone.
      const installDirExists = await Bun.file(
        join(projDir, ".agents", "skills", "standalone-skill", "SKILL.md"),
      ).exists();
      expect(installDirExists).toBe(false);

      // state.json must no longer reference the skill.
      const stateText = await readFile(
        join(projDir, ".agents", "state.json"),
        "utf8",
      );
      const state = JSON.parse(stateText) as {
        version: number;
        skills: Array<{ name: string }>;
      };
      expect(state.skills.some((s) => s.name === "standalone-skill")).toBe(
        false,
      );

      // The manifest file itself must be unchanged (sync only updates state).
      const manifestText = await readFile(
        join(projDir, "skilltap.toml"),
        "utf8",
      );
      expect(manifestText.trim()).toBe("");
    } finally {
      await removeTmpDir(projDir);
      await removeTmpDir(projHome);
      await removeTmpDir(projConfig);
    }
  });
});

describe("sync -- strict: stops on first failure", () => {
  let skillRepo: { path: string; cleanup: () => Promise<void> };

  beforeAll(async () => {
    skillRepo = await createStandaloneSkillRepo();
  });

  afterAll(async () => {
    await skillRepo.cleanup();
  });

  test("Test 7: --strict exits non-zero and halts after the first failing add", async () => {
    // Two skills declared: one whose path sorts alphabetically BEFORE the valid
    // one (so it's encountered first and fails), then the real fixture path.
    // We choose "/aaa/nonexistent" — "/aaa" sorts before any real tmp path.
    const badSource = "/aaa/nonexistent-skill-that-does-not-exist";
    const proj = await makeProjectRoot({
      // smol-toml inline tables: order of keys in a TOML file is preserved.
      // badSource sorts before skillRepo.path alphabetically, but we lay them
      // out explicitly so the bad one appears first in the file.
      manifestToml: `[skills]\n"${badSource}" = "*"\n"${skillRepo.path}" = "*"\n`,
      lockfileToml: `version = 1\n\n[[skill]]\nsource = "${badSource}"\nref = "main"\nrange = "*"\n\n[[skill]]\nsource = "${skillRepo.path}"\nref = "main"\nrange = "*"\n`,
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode } = await runSkilltap(
        ["sync", "--apply", "--strict", "--skip-scan"],
        homeDir,
        configDir,
        proj.path,
      );
      // --strict must exit non-zero on failure.
      expect(exitCode).not.toBe(0);

      // Because strict stops at first failure, at most one skill is installed.
      // The valid skill (standalone-skill) must NOT be installed because strict
      // halted before reaching it.
      const validInstalled = await Bun.file(
        join(proj.path, ".agents", "skills", "standalone-skill", "SKILL.md"),
      ).exists();
      expect(validInstalled).toBe(false);
    } finally {
      await proj.cleanup();
    }
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Adversarial / schema-validation tests (A2, A3, A18, A19)
// ──────────────────────────────────────────────────────────────────────────────

describe("sync — adversarial: invalid manifest schema (A2)", () => {
  test("A2: TOML with non-bool components value errors with a Zod validation message", async () => {
    // smol-toml parses this cleanly; Zod rejects it because components values
    // must be boolean, not string. The error must surface to stderr and the
    // command must exit 1 without writing any files.
    const proj = await makeProjectRoot({
      manifestToml: `[plugins]\n"github:example/x" = { components = { "test-skipper" = "true" } }\n`,
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(1);
      // The error message must reference the invalid field path. The Zod
      // prettifyError output for this schema produces:
      //   "Invalid skilltap.toml: ✖ Invalid input\n  → at plugins[...]"
      expect(stderr).toContain("Invalid");
      expect(stderr.toLowerCase()).toMatch(/toml|manifest|plugins/);
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync — adversarial: invalid lockfile version (A3)", () => {
  test("A3: lockfile with version = 2 is rejected with a schema error", async () => {
    // LockfileSchema uses z.literal(1) for version, so version=2 fails
    // validation. The loadLockfile function surfaces this as a UserError.
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 2\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stderr } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(1);
      // Zod prettifyError for z.literal(1) with value 2 produces:
      //   "Invalid skilltap.lock: ✖ Invalid input: expected 1\n  → at version"
      expect(stderr).toContain("Invalid");
      expect(stderr.toLowerCase()).toMatch(/lock|version/);
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync — adversarial: lock-stale sha mismatch (A18)", () => {
  let skillRepo: { path: string; cleanup: () => Promise<void> };

  beforeAll(async () => {
    skillRepo = await createStandaloneSkillRepo();
  });

  afterAll(async () => {
    await skillRepo.cleanup();
  });

  test("A18a: lock-stale appears in read-only drift report", async () => {
    // Bootstrap a real install so the lockfile has the real sha.
    // Then overwrite the lockfile sha with a known-bad value.
    const { projDir, projHome, projConfig } =
      await bootstrapProjectWithSkill(skillRepo.path);
    try {
      const lockText = await readFile(
        join(projDir, "skilltap.lock"),
        "utf8",
      );
      // Replace the real sha with a fake one. The lockfile stores sha as a
      // string value in a TOML [[skill]] entry.
      const patchedLock = lockText.replace(/sha = "[^"]*"/, 'sha = "deadbeefdeadbeef"');
      await writeFile(join(projDir, "skilltap.lock"), patchedLock);

      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        projHome,
        projConfig,
        projDir,
      );
      expect(exitCode).toBe(0);
      // The drift report must mention "lock stale" (the kindLabel renders it as
      // "⚠ lock stale" per sync.ts line 210).
      expect(stdout).toContain("lock stale");
      expect(stdout).toContain("drift report");
    } finally {
      await removeTmpDir(projDir);
      await removeTmpDir(projHome);
      await removeTmpDir(projConfig);
    }
  });

  test("A18b: sync --apply skips lock-stale items (apply.ts skips lock-* kinds)", async () => {
    // Per apply.test.ts: "lock-missing/lock-stale/lock-orphan all count as skipped".
    // Apply must exit 0 and report skipped:1.
    const { projDir, projHome, projConfig } =
      await bootstrapProjectWithSkill(skillRepo.path);
    try {
      const lockText = await readFile(
        join(projDir, "skilltap.lock"),
        "utf8",
      );
      const patchedLock = lockText.replace(/sha = "[^"]*"/, 'sha = "deadbeefdeadbeef"');
      await writeFile(join(projDir, "skilltap.lock"), patchedLock);

      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--apply", "--skip-scan"],
        projHome,
        projConfig,
        projDir,
      );
      expect(exitCode).toBe(0);
      // The apply summary prints "N skipped". With 1 lock-stale item the
      // summary line is "1 skipped".
      expect(stdout).toContain("skipped");

      // Lockfile sha must still be "deadbeefdeadbeef" — apply does not rewrite
      // lock-* entries.
      const lockAfter = await readFile(
        join(projDir, "skilltap.lock"),
        "utf8",
      );
      expect(lockAfter).toContain("deadbeefdeadbeef");
    } finally {
      await removeTmpDir(projDir);
      await removeTmpDir(projHome);
      await removeTmpDir(projConfig);
    }
  });
});

describe("sync — adversarial: lock-orphan (A19)", () => {
  test("A19a: lock-orphan surfaces in drift report", async () => {
    // Lockfile has an entry that has no matching manifest declaration or state
    // record — this is a lock-orphan.
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml:
        'version = 1\n\n[[skill]]\nsource = "github:example/orphan"\nref = "v1.0"\nrange = "*"\n',
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("drift report");
      // kindLabel renders "lock-orphan" as "? lock orphan"
      expect(stdout).toContain("lock orphan");
      expect(stdout).toContain("skilltap sync --apply");
    } finally {
      await proj.cleanup();
    }
  });

  test("A19b: sync --apply skips lock-orphan items and exits 0", async () => {
    // Per apply.test.ts: lock-orphan is counted as skipped, not applied.
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml:
        'version = 1\n\n[[skill]]\nsource = "github:example/orphan"\nref = "v1.0"\nrange = "*"\n',
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--apply", "--skip-scan"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      // Apply summary must mention skipped (1 skipped, 0 applied, 0 failed).
      expect(stdout).toContain("skipped");
    } finally {
      await proj.cleanup();
    }
  });
});
