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
import { loadLockfile, loadManifest } from "@skilltap/core";
import {
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

const CLI_ENTRY = `${import.meta.dir}/../src/index.ts`;

let homeDir: string;
let configDir: string;
let projectRoot: string;
let skillRepo: { path: string; cleanup: () => Promise<void> };

async function run(
  args: string[],
  opts: { cwd?: string } = {},
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["bun", "run", "--bun", CLI_ENTRY, ...args], {
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

  // ── 7. --agent flag — Phase 31c-c-2c follow-up ───────────────────────────────

  test("7. --agent flag forces non-interactive output even outside TTY", async () => {
    // Use a fresh project for this — independent of the prior sequence.
    // Note: --agent requires security scanning by default, so we can't
    // also pass --skip-scan. The standalone-skill fixture is small and
    // passes the static scan cleanly.
    const agentDir = await makeTmpDir();
    try {
      await initRepo(agentDir);
      await writeFile(join(agentDir, "skilltap.toml"), "");
      const { exitCode, stdout, stderr } = await run(
        ["install", skillRepo.path, "--project", "--agent"],
        { cwd: agentDir },
      );
      if (exitCode !== 0) {
        // eslint-disable-next-line no-console
        console.error("agent-install stdout:", stdout);
        // eslint-disable-next-line no-console
        console.error("agent-install stderr:", stderr);
      }
      expect(exitCode).toBe(0);
      // Agent mode emits "OK: Installed <name>" plain-text lines (vs the
      // clack spinner output of interactive mode).
      expect(stdout).toContain("OK: Installed standalone-skill");
    } finally {
      await removeTmpDir(agentDir);
    }
  });
});
