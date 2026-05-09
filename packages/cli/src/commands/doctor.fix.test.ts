import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";

setDefaultTimeout(60_000);

import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  createTestEnv,
  initRepo,
  pathExists,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

// ─── Shared state ─────────────────────────────────────────────────────────────

let env: TestEnv;
let homeDir: string;
let configDir: string;
// skilltapDir = configDir/skilltap — where state.json/installed.json live
let skilltapDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
  skilltapDir = join(configDir, "skilltap");
  await mkdir(skilltapDir, { recursive: true });
});

afterEach(async () => {
  await env.cleanup();
});

// ─── Test A4: corrupt state.json reported without --fix ───────────────────────
//
// Verifies that doctor (no --fix) surfaces the corrupt state.json as a failure
// AND leaves the file completely untouched (no auto-repair).

describe("doctor — corrupt state.json without --fix (Test A4)", () => {
  test("exits non-zero, marks state.json check as fail in JSON output, hints about --fix, and does not touch the file", async () => {
    const statePath = join(skilltapDir, "state.json");
    const corruptContent = "{not valid}";
    await writeFile(statePath, corruptContent);

    // Capture the raw bytes before running doctor
    const before = await readFile(statePath, "utf8");

    const { exitCode, stdout } = await runSkilltap(
      ["doctor", "--json"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(1);

    const result = JSON.parse(stdout) as {
      ok: boolean;
      checks: Array<{
        name: string;
        status: string;
        issues?: Array<{ message: string; fixable: boolean }>;
      }>;
    };
    expect(result.ok).toBe(false);

    const stateCheck = result.checks.find((c) => c.name === "state.json");
    expect(stateCheck).toBeDefined();
    expect(stateCheck!.status).toBe("fail");

    // At least one issue should be fixable (the corrupt-JSON issue)
    const fixableIssue = stateCheck?.issues?.find((i) => i.fixable);
    expect(fixableIssue).toBeDefined();

    // The interactive output should mention --fix as available
    const { stdout: interactiveOut } = await runSkilltap(
      ["doctor"],
      homeDir,
      configDir,
    );
    expect(interactiveOut).toContain("--fix");

    // File must be byte-identical — no auto-repair without --fix
    const after = await readFile(statePath, "utf8");
    expect(after).toBe(before);
  });
});

// ─── Test 23: doctor --fix repairs corrupt state.json ─────────────────────────
//
// Behavior verified from state-v2.test.ts:
//   fix() copies the file to state.json.bak, then writes a fresh
//   { version: 2, skills: [], plugins: [], mcpServers: [] } in its place.

describe("doctor --fix — corrupt state.json (Test 23)", () => {
  test("repairs state.json (backup + fresh v2 file), reports the fix applied, and exits 0 when every fix succeeds", async () => {
    const statePath = join(skilltapDir, "state.json");
    await writeFile(statePath, "{not valid}");

    const { exitCode, stdout } = await runSkilltap(
      ["doctor", "--fix"],
      homeDir,
      configDir,
    );

    // After Unit 3.18: when every fix succeeds, doctor exits 0.
    expect(exitCode).toBe(0);

    // A backup must exist
    expect(await pathExists(`${statePath}.bak`)).toBe(true);

    // The repaired file must parse as valid JSON with version: 2
    const repaired = await Bun.file(statePath)
      .json()
      .catch(() => null);
    expect(repaired).not.toBeNull();
    expect(repaired.version).toBe(2);
    expect(Array.isArray(repaired.skills)).toBe(true);

    // Doctor output must show the fix was applied (✓ marker on the issue)
    expect(stdout).toContain("✓");

    // After Unit 3.18: JSON output reports the fixed check as status: "pass".
    // Reset the file so the second run also exercises the fix path cleanly.
    await writeFile(statePath, "{not valid}");

    const { stdout: jsonOut } = await runSkilltap(
      ["doctor", "--json", "--fix"],
      homeDir,
      configDir,
    );
    const parsed = JSON.parse(jsonOut) as {
      ok: boolean;
      checks: Array<{
        name: string;
        status: string;
        fixed?: boolean;
        issues?: Array<{ fixed?: boolean; fixDescription?: string }>;
      }>;
    };
    expect(parsed.ok).toBe(true);
    const stateCheck = parsed.checks.find((c) => c.name === "state.json");
    expect(stateCheck).toBeDefined();
    expect(stateCheck!.status).toBe("pass");
    expect(stateCheck!.fixed).toBe(true);
    expect(stateCheck!.issues?.[0]?.fixed).toBe(true);
  });
});

// ─── Test 24: doctor --fix prunes orphan MCP entries ─────────────────────────
//
// Behavior verified from mcp-consistency.test.ts:
//   When a `skilltap:*` key in the agent config has no matching state record,
//   the check raises a fixable "Orphan" issue. fix() calls removeMcpServers()
//   which deletes the key from the JSON file.
//
// globalBase() == SKILLTAP_HOME so the claude-code config lives at:
//   <homeDir>/.claude/settings.json
//
// The mcp-consistency check only runs when state is non-null (requires a
// valid state.json). An empty-but-valid state.json is sufficient.

describe("doctor --fix — orphan MCP entry (Test 24)", () => {
  test("exits 0, removes orphan skilltap: key from agent config, and reports the prune", async () => {
    // Plant an orphan MCP entry in the global claude-code config.
    // No corresponding plugin record exists in state.json.
    const claudeConfigDir = join(homeDir, ".claude");
    await mkdir(claudeConfigDir, { recursive: true });
    const settingsPath = join(claudeConfigDir, "settings.json");
    const orphanKey = "skilltap:nonexistent-plugin:nonexistent-server";
    await writeFile(
      settingsPath,
      JSON.stringify(
        {
          mcpServers: {
            [orphanKey]: { command: "echo", args: [] },
          },
        },
        null,
        2,
      ),
    );

    // Write a valid but empty state.json so the mcp-consistency check activates.
    // (When state is null the check returns n/a and the orphan goes undetected.)
    const statePath = join(skilltapDir, "state.json");
    await writeFile(
      statePath,
      JSON.stringify(
        { version: 2, skills: [], plugins: [], mcpServers: [] },
        null,
        2,
      ),
    );

    const { exitCode, stdout } = await runSkilltap(
      ["doctor", "--fix"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);

    // The orphan key must be absent from the settings file after the fix
    const settingsAfter = JSON.parse(await readFile(settingsPath, "utf8")) as {
      mcpServers?: Record<string, unknown>;
    };
    expect(settingsAfter.mcpServers?.[orphanKey]).toBeUndefined();

    // Doctor output should show a fix was applied (✓ marker)
    expect(stdout).toContain("✓");
  });
});

// ─── Test 25: doctor --fix renames v0.x installed.json to .v1.bak ─────────────
//
// Behavior verified from v1-orphans.test.ts:
//   The check only fires when state has populated content (skills > 0 or
//   plugins > 0) at the time checkV1Orphans is called. fix() renames
//   installed.json → installed.json.v1.bak.
//
// Subtlety: checkSkills runs before checkStateV2. If a skill in state.json
// has no corresponding directory on disk, checkSkills removes it (fix applied
// automatically), leaving state empty. checkStateV2 then reads an empty state
// → checkV1Orphans sees no populated state → returns n/a.
//
// Solution: create the actual skill directory on disk so checkSkills doesn't
// prune the record, leaving state populated for checkV1Orphans.
//
// The test uses a project-scoped setup (git repo as cwd) so doctor's
// tryFindProjectRoot picks up the project and checks .agents/installed.json.

describe("doctor --fix — v0.x installed.json renamed to .v1.bak (Test 25)", () => {
  test("exits 0, renames project installed.json to installed.json.v1.bak, leaves state.json intact", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-fix-v1-test-"));

    try {
      await initRepo(projectRoot);

      const agentsDir = join(projectRoot, ".agents");
      await mkdir(agentsDir, { recursive: true });

      // Create the actual skill directory so checkSkills doesn't prune the
      // record — otherwise state is empty by the time checkV1Orphans runs.
      const skillDir = join(agentsDir, "skills", "some-skill");
      await mkdir(skillDir, { recursive: true });

      // Write a populated project state.json so v1-orphans check activates.
      const projectStatePath = join(agentsDir, "state.json");
      const populatedState = {
        version: 2,
        skills: [
          {
            name: "some-skill",
            description: "",
            repo: "https://github.com/n/r",
            ref: "main",
            sha: "abc123",
            scope: "project",
            path: null,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            active: true,
          },
        ],
        plugins: [],
        mcpServers: [],
      };
      await writeFile(
        projectStatePath,
        JSON.stringify(populatedState, null, 2),
      );

      // Plant the leftover v0.x installed.json in the project .agents dir
      const installedPath = join(agentsDir, "installed.json");
      await writeFile(
        installedPath,
        JSON.stringify({ version: 1, skills: [] }, null, 2),
      );

      // Capture state.json content before the fix to verify it is unchanged
      const stateBefore = await readFile(projectStatePath, "utf8");

      const { exitCode, stdout } = await runSkilltap(
        ["doctor", "--fix"],
        homeDir,
        configDir,
        projectRoot, // run with cwd = projectRoot so doctor finds the project
      );

      expect(exitCode).toBe(0);

      // installed.json must be gone; its .v1.bak must exist
      expect(await pathExists(installedPath)).toBe(false);
      expect(await pathExists(`${installedPath}.v1.bak`)).toBe(true);

      // state.json must be byte-identical to what we wrote (no mutation)
      const stateAfter = await readFile(projectStatePath, "utf8");
      expect(stateAfter).toBe(stateBefore);

      // Doctor output should show a fix was applied (✓ marker)
      expect(stdout).toContain("✓");
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });
});

// ─── Test 26: doctor --fix recovers a corrupt skilltap.toml ───────────────────
//
// Manifest-drift check now treats loadManifest failures as fixable: backup the
// corrupt file to skilltap.toml.bak and write a fresh empty manifest. This is
// the same recovery path the install preflight uses in interactive mode.

describe("doctor --fix — corrupt skilltap.toml (Test 26)", () => {
  test("backs up the corrupt manifest to .bak and writes a fresh empty manifest", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-doctor-"));
    try {
      await initRepo(projectRoot);

      const manifestPath = join(projectRoot, "skilltap.toml");
      const brokenContent = '[skills\nthis = "is a broken table\n';
      await writeFile(manifestPath, brokenContent);

      // doctor needs a project-rooted state.json so the manifest-drift check
      // actually runs (see manifest-drift.ts: it returns 'n/a' when state is null).
      await mkdir(join(projectRoot, ".agents"), { recursive: true });
      await writeFile(
        join(projectRoot, ".agents", "state.json"),
        JSON.stringify({ version: 2, skills: [], plugins: [] }),
      );

      const { exitCode, stdout } = await runSkilltap(
        ["doctor", "--fix"],
        homeDir,
        configDir,
        projectRoot,
      );

      // After fix: original content survives at .bak, manifest now parseable.
      const bakAfter = await readFile(`${manifestPath}.bak`, "utf8");
      expect(bakAfter).toBe(brokenContent);

      const manifestAfter = await readFile(manifestPath, "utf8");
      // Fresh manifest is empty (all-defaults). loadManifest will succeed on it.
      expect(manifestAfter).toBe("");

      // Doctor surfaces the fix (✓ marker on fixed issue).
      expect(stdout).toContain("✓");

      // After Unit 3.18: doctor exits 0 when every fix succeeds.
      expect(exitCode).toBe(0);
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });
});

// ─── Test 27: --fix exit-0-after-success regression lock (Unit 3.18) ──────────
//
// Before Unit 3.18, doctor would exit 1 even when every fixable issue had been
// repaired, because hasFailure was computed from the unmodified check status.
// This test pins the new contract: when every failing check has c.fixed = true,
// doctor exits 0 and the JSON output reports status: "pass" + fixed: true on
// each repaired check.

describe("doctor --fix — exit code 0 when all fixes succeed (Unit 3.18)", () => {
  test("planted corrupt state.json + missing dirs all repair → exit 0, JSON status flips to pass", async () => {
    const statePath = join(skilltapDir, "state.json");
    await writeFile(statePath, "{not valid}");

    const { exitCode, stdout } = await runSkilltap(
      ["doctor", "--json", "--fix"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);

    const parsed = JSON.parse(stdout) as {
      ok: boolean;
      checks: Array<{
        name: string;
        status: string;
        fixed?: boolean;
        fixDescription?: string;
        issues?: Array<{ message: string; fixable: boolean; fixed?: boolean }>;
      }>;
    };
    expect(parsed.ok).toBe(true);

    const stateCheck = parsed.checks.find((c) => c.name === "state.json");
    expect(stateCheck).toBeDefined();
    expect(stateCheck!.status).toBe("pass");
    expect(stateCheck!.fixed).toBe(true);
    expect(stateCheck!.fixDescription).toBeDefined();

    // Confirm all reported issues across checks were resolved.
    for (const check of parsed.checks) {
      if (check.issues) {
        for (const issue of check.issues) {
          if (issue.fixable) {
            expect(issue.fixed).toBe(true);
          }
        }
      }
    }
  });
});
