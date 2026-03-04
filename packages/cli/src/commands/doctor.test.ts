import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(45_000);
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runDoctor(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "doctor", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  delete process.env.SKILLTAP_HOME;
  delete process.env.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

// ─── Healthy environment ──────────────────────────────────────────────────────

describe("doctor — healthy environment", () => {
  test("exits 0 on fresh environment", async () => {
    const { exitCode } = await runDoctor([], homeDir, configDir);
    // Fresh env has warnings (missing dirs) but no failures
    expect(exitCode).toBe(0);
  });

  test("shows check names in output", async () => {
    const { stdout } = await runDoctor([], homeDir, configDir);
    expect(stdout).toContain("git");
    expect(stdout).toContain("config");
    expect(stdout).toContain("dirs");
    expect(stdout).toContain("installed");
    expect(stdout).toContain("skills");
    expect(stdout).toContain("symlinks");
    expect(stdout).toContain("taps");
    expect(stdout).toContain("agents");
  });

  test("shows 'Everything looks good!' when all pass", async () => {
    // Create all required dirs so everything passes
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(join(skilltapDir, "cache"), { recursive: true });
    await mkdir(join(skilltapDir, "taps"), { recursive: true });
    await mkdir(join(homeDir, ".agents", "skills"), { recursive: true });
    await writeFile(
      join(skilltapDir, "config.toml"),
      '[defaults]\nalso = []\nyes = false\nscope = ""\n[security]\nscan = "static"\non_warn = "prompt"\nrequire_scan = false\nagent = ""\nthreshold = 5\nmax_size = 51200\nollama_model = ""\n["agent-mode"]\nenabled = false\nscope = "project"\n',
    );

    const { exitCode, stdout } = await runDoctor([], homeDir, configDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("Everything looks good!");
  });
});

// ─── Broken environment ───────────────────────────────────────────────────────

describe("doctor — broken state", () => {
  test("exits 1 when config.toml has invalid TOML", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(join(skilltapDir, "config.toml"), "not valid toml ===\n");

    const { exitCode, stdout } = await runDoctor([], homeDir, configDir);
    expect(exitCode).toBe(1);
    expect(stdout).toContain("invalid TOML");
  });

  test("shows issue count in summary", async () => {
    const { stdout } = await runDoctor([], homeDir, configDir);
    // Fresh env has missing dirs as warnings
    expect(stdout).toMatch(/issue(s)? found/);
  });
});

// ─── --fix flag ───────────────────────────────────────────────────────────────

describe("doctor --fix", () => {
  test("creates missing directories", async () => {
    const { exitCode } = await runDoctor(["--fix"], homeDir, configDir);
    expect(exitCode).toBe(0);

    // Verify dirs were created
    const skilltapDir = join(configDir, "skilltap");
    const { stat } = await import("node:fs/promises");
    const cacheExists = await stat(join(skilltapDir, "cache"))
      .then((s) => s.isDirectory())
      .catch(() => false);
    expect(cacheExists).toBe(true);
  });

  test("shows fixed issues in output", async () => {
    const { stdout } = await runDoctor(["--fix"], homeDir, configDir);
    // Should show some fix messages
    expect(stdout).toContain("✓");
  });

  test("repairs corrupt installed.json", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    const installedFile = join(skilltapDir, "installed.json");
    await writeFile(installedFile, "not json {{{}}}");

    const { exitCode } = await runDoctor(["--fix"], homeDir, configDir);
    // After fix, the installed.json failure is resolved
    // Still may exit 1 if other failures exist but installed.json itself is now fixed
    const repaired = await Bun.file(installedFile).json().catch(() => null);
    expect(repaired).not.toBeNull();
    expect(repaired.version).toBe(1);
  });
});

// ─── --json flag ─────────────────────────────────────────────────────────────

describe("doctor --json", () => {
  test("outputs valid JSON", async () => {
    const { stdout } = await runDoctor(["--json"], homeDir, configDir);
    let parsed: unknown;
    expect(() => {
      parsed = JSON.parse(stdout);
    }).not.toThrow();

    const result = parsed as { ok: boolean; checks: unknown[] };
    expect(typeof result.ok).toBe("boolean");
    expect(Array.isArray(result.checks)).toBe(true);
  });

  test("JSON includes all checks", async () => {
    const { stdout } = await runDoctor(["--json"], homeDir, configDir);
    const result = JSON.parse(stdout) as {
      ok: boolean;
      checks: Array<{ name: string; status: string }>;
    };

    const names = result.checks.map((c) => c.name);
    expect(names).toContain("git");
    expect(names).toContain("config");
    expect(names).toContain("dirs");
    expect(names).toContain("installed");
    expect(names).toContain("skills");
    expect(names).toContain("symlinks");
    expect(names).toContain("taps");
    expect(names).toContain("agents");
  });

  test("JSON check has status field", async () => {
    const { stdout } = await runDoctor(["--json"], homeDir, configDir);
    const result = JSON.parse(stdout) as {
      checks: Array<{ name: string; status: string }>;
    };

    for (const check of result.checks) {
      expect(["pass", "warn", "fail"]).toContain(check.status);
    }
  });

  test("exits 1 with JSON when failure exists", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(join(skilltapDir, "config.toml"), "not valid toml ===\n");

    const { exitCode, stdout } = await runDoctor(["--json"], homeDir, configDir);
    expect(exitCode).toBe(1);
    const result = JSON.parse(stdout) as { ok: boolean };
    expect(result.ok).toBe(false);
  });

  test("JSON --fix shows fixed field on issues", async () => {
    // Run with fix on a fresh env (missing dirs are fixable)
    const { stdout } = await runDoctor(["--json", "--fix"], homeDir, configDir);
    const result = JSON.parse(stdout) as {
      checks: Array<{
        name: string;
        issues?: Array<{ fixable: boolean; fixed?: boolean }>;
      }>;
    };

    const dirsCheck = result.checks.find((c) => c.name === "dirs");
    if (dirsCheck?.issues) {
      for (const issue of dirsCheck.issues) {
        if (issue.fixable) {
          expect(issue.fixed).toBe(true);
        }
      }
    }
  });
});
