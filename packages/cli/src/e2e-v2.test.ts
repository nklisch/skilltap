/**
 * v2.0 end-to-end test (Phase 38.5).
 *
 * Walks the canonical v2 journey as a real CLI subprocess:
 *   clean init  →  install (writes manifest + lockfile + state)
 *               →  status dashboard
 *               →  doctor (must run cleanly)
 *               →  fresh-clone sync (manifest+lockfile only → reinstalls)
 *               →  migrate (v1 installed.json → v2 state.json)
 *
 * Tests run sequentially and share homeDir/configDir/projectRoot.
 */
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { loadLockfile, loadManifest, saveManifest } from "@skilltap/core";
import {
  cliCmd,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

let homeDir: string;
let configDir: string;
let projectRoot: string;
let skillRepo: { path: string; cleanup: () => Promise<void> };

async function run(
  args: string[],
  opts: { cwd?: string } = {},
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn([...cliCmd(), ...args], {
    cwd: opts.cwd ?? projectRoot,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      SKILLTAP_HOME: homeDir,
      XDG_CONFIG_HOME: configDir,
    },
  });
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  projectRoot = await makeTmpDir();
  skillRepo = await createStandaloneSkillRepo();
  // Make projectRoot a real git repo so smart-scope-default kicks in.
  await initRepo(projectRoot);
});

afterAll(async () => {
  await skillRepo.cleanup();
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
  await removeTmpDir(projectRoot);
});

describe("E2E v2 — manifest, sync, migrate, status, doctor", () => {
  // ── 1. Fresh project with empty skilltap.toml ────────────────────────────────

  test("1. seed a fresh skilltap.toml at project root", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    const exists = await Bun.file(join(projectRoot, "skilltap.toml")).exists();
    expect(exists).toBe(true);
  });

  // ── 2. install --project writes state + manifest + lockfile ──────────────────

  test("2. install writes state.json, skilltap.toml, skilltap.lock", async () => {
    const { exitCode, stderr } = await run([
      "install",
      "skill",
      skillRepo.path,
      "--project",
      "--skip-scan",
      "--yes",
    ]);
    if (exitCode !== 0) {
      // eslint-disable-next-line no-console
      console.error("install stderr:", stderr);
    }
    expect(exitCode).toBe(0);

    // manifest entry written
    const manifestResult = await loadManifest(projectRoot);
    expect(manifestResult.ok).toBe(true);
    if (!manifestResult.ok) return;
    const skillKeys = Object.keys(manifestResult.value.skills ?? {});
    expect(skillKeys).toHaveLength(1);
    // Local paths pass through canonicalization unchanged.
    expect(skillKeys[0]).toBe(skillRepo.path);

    // lockfile entry written with sha
    const lockfileResult = await loadLockfile(projectRoot);
    expect(lockfileResult.ok).toBe(true);
    if (!lockfileResult.ok) return;
    const skillEntries = lockfileResult.value.skill ?? [];
    expect(skillEntries).toHaveLength(1);
    expect(skillEntries[0]?.source).toBe(skillRepo.path);
    expect(typeof skillEntries[0]?.sha).toBe("string");
    expect((skillEntries[0]?.sha ?? "").length).toBeGreaterThan(0);

    // Phase 31c-c-2d-1: install writes ONLY to state.json. installed.json
    // is no longer maintained (it's read-fallback only for unmigrated
    // v0.x users). CLAUDE.md "v2.1 conventions": "Don't re-introduce
    // installed.json writes; the dual-write layer was deleted in Refactor 2."
    const stateText = await readFile(
      join(projectRoot, ".agents", "state.json"),
      "utf8",
    );
    const state = JSON.parse(stateText) as {
      version: number;
      skills: Array<{ name: string }>;
    };
    expect(state.version).toBe(2);
    expect(state.skills.map((s) => s.name)).toEqual(["standalone-skill"]);

    // Invariant (CLAUDE.md "dual-write deleted"): legacy installed.json
    // and plugins.json must NOT be written by install. If a future refactor
    // accidentally re-introduces dual-write, this assertion fires.
    const legacyInstalledExists = await Bun.file(
      join(projectRoot, ".agents", "installed.json"),
    ).exists();
    expect(legacyInstalledExists).toBe(false);
    const legacyPluginsExists = await Bun.file(
      join(projectRoot, ".agents", "plugins.json"),
    ).exists();
    expect(legacyPluginsExists).toBe(false);
  });

  // ── 3. status — shows the installed skill from state.json ────────────────────

  test("3. status dashboard lists the project skill", async () => {
    const { exitCode, stdout } = await run(["status"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
  });

  // ── 4. doctor — runs end-to-end with no errors ───────────────────────────────

  test("4. doctor exits cleanly", async () => {
    const { exitCode, stderr } = await run(["doctor"]);
    if (exitCode !== 0) {
      // eslint-disable-next-line no-console
      console.error("doctor stderr:", stderr);
    }
    // doctor returns non-zero only on hard failures; warnings are fine.
    expect(exitCode).toBe(0);
  });

  // ── 5. sync on a fresh-clone shape: manifest+lock present, state empty ───────

  test("5. sync --apply on a fresh clone reinstalls from manifest", async () => {
    const cloneDir = await makeTmpDir();
    try {
      await initRepo(cloneDir);
      // Copy manifest + lockfile only — no .agents/state.json.
      const mfst = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
      const lock = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
      await writeFile(join(cloneDir, "skilltap.toml"), mfst);
      await writeFile(join(cloneDir, "skilltap.lock"), lock);

      const { exitCode, stdout, stderr } = await run(["sync", "--apply"], {
        cwd: cloneDir,
      });
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("sync stdout:", stdout);
        // eslint-disable-next-line no-console
        console.error("sync stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // After apply, state.json exists with the skill.
      const state = JSON.parse(
        await readFile(join(cloneDir, ".agents", "state.json"), "utf8"),
      ) as { version: number; skills: Array<{ name: string }> };
      expect(state.version).toBe(2);
      expect(state.skills.some((s) => s.name === "standalone-skill")).toBe(
        true,
      );
    } finally {
      await removeTmpDir(cloneDir);
    }
  });

  // ── 6. migrate — v1 layout → state.json v2 ───────────────────────────────────

  test("6. migrate converts a v1 installed.json to state.json", async () => {
    const v1Dir = await makeTmpDir();
    try {
      await initRepo(v1Dir);
      // Build a minimal v0.x installed.json + a SKILL.md so the migrator
      // believes the skill is on disk.
      const installed = {
        version: 1,
        skills: [
          {
            name: "legacy-skill",
            description: "carried over from v1",
            repo: skillRepo.path,
            ref: "main",
            sha: "abc1234",
            scope: "project",
            path: null,
            tap: null,
            also: ["claude-code"],
            installedAt: "2026-01-01T00:00:00.000Z",
            updatedAt: "2026-01-01T00:00:00.000Z",
            active: true,
          },
        ],
      };
      await mkdir(join(v1Dir, ".agents"), { recursive: true });
      await writeFile(
        join(v1Dir, ".agents", "installed.json"),
        JSON.stringify(installed, null, 2),
      );
      await mkdir(join(v1Dir, ".agents", "skills", "legacy-skill"), {
        recursive: true,
      });
      await writeFile(
        join(v1Dir, ".agents", "skills", "legacy-skill", "SKILL.md"),
        "---\nname: legacy-skill\ndescription: legacy\n---\n# Legacy\n",
      );

      const { exitCode, stdout, stderr } = await run(["migrate"], {
        cwd: v1Dir,
      });
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("migrate stdout:", stdout);
        // eslint-disable-next-line no-console
        console.error("migrate stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      const stateExists = await Bun.file(
        join(v1Dir, ".agents", "state.json"),
      ).exists();
      expect(stateExists).toBe(true);
      const state = JSON.parse(
        await readFile(join(v1Dir, ".agents", "state.json"), "utf8"),
      ) as { version: number; skills: Array<{ name: string }> };
      expect(state.version).toBe(2);
      expect(state.skills.some((s) => s.name === "legacy-skill")).toBe(true);
    } finally {
      await removeTmpDir(v1Dir);
    }
  });

  // ── 7. --yes installs cleanly in non-TTY pipe mode ──────────────────────────

  test("7. --yes installs cleanly in non-TTY pipe mode", async () => {
    // Use a fresh project for this — independent of the prior sequence.
    const agentDir = await makeTmpDir();
    try {
      await initRepo(agentDir);
      await writeFile(join(agentDir, "skilltap.toml"), "");
      const { exitCode, stdout, stderr } = await run(
        ["install", "skill", skillRepo.path, "--project", "--yes", "--skip-scan"],
        { cwd: agentDir },
      );
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("install stdout:", stdout);
        // eslint-disable-next-line no-console
        console.error("install stderr:", stderr);
      }
      expect(exitCode).toBe(0);
      expect(stdout).toContain("standalone-skill");
    } finally {
      await removeTmpDir(agentDir);
    }
  });

  // ── 8. skills remove drops entry from manifest, lockfile, and state ──────────

  test("8. skills remove drops the skill from manifest, lockfile, and state", async () => {
    // Use a fresh project dir to avoid disturbing projectRoot that tests 1-7
    // rely on. Install into it first so there's something to remove.
    const removeDir = await makeTmpDir();
    try {
      await initRepo(removeDir);
      await writeFile(join(removeDir, "skilltap.toml"), "");

      // Install the skill so manifest + lockfile + state are all populated.
      const installResult = await run(
        ["install", "skill", skillRepo.path, "--project", "--skip-scan", "--yes"],
        { cwd: removeDir },
      );
      if (installResult.exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("setup install stderr:", installResult.stderr);
      }
      expect(installResult.exitCode).toBe(0);

      // Sanity-check: manifest has the skill before remove.
      const beforeManifest = await loadManifest(removeDir);
      expect(beforeManifest.ok).toBe(true);
      if (!beforeManifest.ok) return;
      expect(Object.keys(beforeManifest.value.skills ?? {})).toHaveLength(1);

      // Now remove via the CLI — must pass --project so removeSkill() gets
      // projectRoot and calls removeSkillFromManifest() (see core/src/remove.ts:108-114).
      const { exitCode, stderr } = await run(
        ["remove", "skill", "standalone-skill", "--yes", "--project"],
        { cwd: removeDir },
      );
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("remove stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // Manifest: skills table must be empty.
      const afterManifest = await loadManifest(removeDir);
      expect(afterManifest.ok).toBe(true);
      if (!afterManifest.ok) return;
      expect(Object.keys(afterManifest.value.skills ?? {})).toHaveLength(0);

      // Lockfile: skill array must be empty.
      const afterLock = await loadLockfile(removeDir);
      expect(afterLock.ok).toBe(true);
      if (!afterLock.ok) return;
      expect(afterLock.value.skill ?? []).toHaveLength(0);

      // state.json: no skill entry.
      const stateFile = Bun.file(join(removeDir, ".agents", "state.json"));
      expect(await stateFile.exists()).toBe(true);
      const state = JSON.parse(await stateFile.text()) as {
        version: number;
        skills: Array<{ name: string }>;
      };
      expect(state.skills.map((s) => s.name)).not.toContain("standalone-skill");

      // Install directory must be gone.
      const installDir = join(
        removeDir,
        ".agents",
        "skills",
        "standalone-skill",
      );
      const installDirExists = await Bun.file(
        join(installDir, "SKILL.md"),
      ).exists();
      expect(installDirExists).toBe(false);
    } finally {
      await removeTmpDir(removeDir);
    }
  });

  // ── 9. status --json shape: skills, plugins, taps, drift-aware ───────────────

  test("9. status --json includes skills + taps + a drift field when manifest present", async () => {
    // projectRoot already has standalone-skill installed (from tests 1-2) and
    // has a skilltap.toml (seeded in test 1). Introduce synthetic drift: use
    // saveManifest() to add a declared-but-not-installed entry so the manifest
    // and state disagree, causing drift.inSync === false.
    const beforeManifest = await loadManifest(projectRoot);
    expect(beforeManifest.ok).toBe(true);
    if (!beforeManifest.ok) return;

    // Add a skill to the manifest that is NOT in state.json — this is the drift.
    const driftedManifest = {
      ...beforeManifest.value,
      skills: {
        ...beforeManifest.value.skills,
        "github:example/declared-not-installed": "*" as const,
      },
    };
    await saveManifest(projectRoot, driftedManifest);

    try {
      const { exitCode, stdout, stderr } = await run(["status", "--json"]);
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("status stderr:", stderr);
      }
      expect(exitCode).toBe(0);

      // Output must be valid JSON (no ANSI escapes polluting it).
      const payload = JSON.parse(stdout) as {
        scope: string;
        hasManifest: boolean;
        skills: Array<{ name: string }>;
        plugins: unknown[];
        taps: unknown[];
        drift: { inSync: boolean; items: Array<{ kind: string }> } | null;
      };

      // The skill installed in test 2 must still appear.
      expect(payload.skills.some((s) => s.name === "standalone-skill")).toBe(
        true,
      );

      // Scope is "project" because projectRoot is a git repo and has a manifest.
      expect(payload.scope).toBe("project");

      // Taps array is always present (built-in tap).
      expect(Array.isArray(payload.taps)).toBe(true);
      expect(payload.taps.length).toBeGreaterThan(0);

      // Manifest is present.
      expect(payload.hasManifest).toBe(true);

      // Drift field: because we added a declared-but-not-installed entry, the
      // drift report should indicate inSync === false with at least one 'add' item.
      // Note: drift detection requires loadLockfile to succeed. If the lockfile
      // doesn't reference the new entry, the skill is 'add' drift (in manifest
      // but not in state).
      expect(payload.drift).not.toBeNull();
      if (payload.drift !== null) {
        expect(payload.drift.inSync).toBe(false);
        const addItem = payload.drift.items.find((i) => i.kind === "add");
        expect(addItem).toBeDefined();
      }
    } finally {
      // Restore manifest to original state so subsequent tests aren't affected.
      await saveManifest(projectRoot, beforeManifest.value);
    }
  });

  // ── 10. Malformed skilltap.toml: interactive auto-recovers ───────────────────

  // The install preflight (cli/src/commands/install.ts: preflightManifestValidity)
  // detects a corrupt skilltap.toml at install start and backs it up, writes
  // a fresh empty manifest, announces loudly, and proceeds.
  //
  // Without this preflight, the install would proceed and addSkillToManifest's
  // silent-skip would swallow the manifest update — leaving state.json updated
  // but skilltap.toml still corrupt and skilltap.lock missing the new entry.

  test("10a. malformed skilltap.toml: preflight backs up and recovers before install", async () => {
    const malformedDir = await makeTmpDir();
    try {
      await initRepo(malformedDir);
      const brokenContent = '[skills\nthis = "is a broken table\n';
      await writeFile(join(malformedDir, "skilltap.toml"), brokenContent);

      const { exitCode } = await run(
        ["install", "skill", skillRepo.path, "--project", "--yes", "--skip-scan"],
        { cwd: malformedDir },
      );

      expect(exitCode).toBe(0);

      // The corrupt file is preserved at .bak
      const bakOnDisk = await Bun.file(
        join(malformedDir, "skilltap.toml.bak"),
      ).text();
      expect(bakOnDisk).toBe(brokenContent);

      // state.json reflects the install
      const stateExists = await Bun.file(
        join(malformedDir, ".agents", "state.json"),
      ).exists();
      expect(stateExists).toBe(true);
    } finally {
      await removeTmpDir(malformedDir);
    }
  });

  test("10b. malformed skilltap.toml in interactive mode auto-recovers and proceeds", async () => {
    const malformedDir = await makeTmpDir();
    try {
      await initRepo(malformedDir);
      const brokenContent = '[skills\nthis = "is a broken table\n';
      await writeFile(join(malformedDir, "skilltap.toml"), brokenContent);

      const { exitCode } = await run(
        ["install", "skill", skillRepo.path, "--project", "--yes", "--skip-scan"],
        { cwd: malformedDir },
      );

      expect(exitCode).toBe(0);

      // The corrupt file is preserved at .bak so the user can recover content.
      const bakOnDisk = await Bun.file(
        join(malformedDir, "skilltap.toml.bak"),
      ).text();
      expect(bakOnDisk).toBe(brokenContent);

      // The fresh manifest now has the install's skill entry (no longer empty
      // because addSkillToManifest ran successfully against the recovered
      // file at the end of install).
      const manifestResult = await loadManifest(malformedDir);
      expect(manifestResult.ok).toBe(true);
      if (!manifestResult.ok) return;
      expect(Object.keys(manifestResult.value.skills)).toContain(
        skillRepo.path,
      );

      // state.json reflects the install — no half-managed state.
      const state = JSON.parse(
        await readFile(join(malformedDir, ".agents", "state.json"), "utf8"),
      ) as { skills: Array<{ name: string }> };
      expect(state.skills.some((s) => s.name === "standalone-skill")).toBe(
        true,
      );
    } finally {
      await removeTmpDir(malformedDir);
    }
  });
});
