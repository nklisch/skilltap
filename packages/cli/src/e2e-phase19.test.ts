/**
 * End-to-end test for Phase 19 commands: create, verify, doctor, completions.
 * Tests run sequentially and share state via homeDir/configDir/workDir.
 */
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { lstat } from "node:fs/promises";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/..`;

let homeDir: string;
let configDir: string;
let workDir: string;

async function run(
  args: string[],
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["bun", "run", "--bun", "src/index.ts", ...args], {
    cwd: CLI_DIR,
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
  workDir = await makeTmpDir();
});

afterAll(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
  await removeTmpDir(workDir);
});

describe("E2E Phase 19 — create → verify → doctor → completions", () => {
  // ── create ──────────────────────────────────────────────────────────────────

  test("1. create basic — non-interactive (name + --template)", async () => {
    const skillDir = join(workDir, "my-e2e-skill");
    const { exitCode, stdout, stderr } = await run([
      "create",
      "my-e2e-skill",
      "--template",
      "basic",
      "--dir",
      skillDir,
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/created|my-e2e-skill/i);
  });

  test("2. create basic — SKILL.md written to disk", async () => {
    const skillMd = join(workDir, "my-e2e-skill", "SKILL.md");
    const stat = await lstat(skillMd).catch(() => null);
    expect(stat).not.toBeNull();
    expect(stat?.isFile()).toBe(true);
  });

  test("3. create npm — non-interactive", async () => {
    const skillDir = join(workDir, "my-npm-skill");
    const { exitCode, stdout, stderr } = await run([
      "create",
      "my-npm-skill",
      "--template",
      "npm",
      "--dir",
      skillDir,
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/created|my-npm-skill/i);
  });

  test("4. create npm — SKILL.md and package.json written", async () => {
    const skillMd = join(workDir, "my-npm-skill", "SKILL.md");
    const pkgJson = join(workDir, "my-npm-skill", "package.json");
    const [skillStat, pkgStat] = await Promise.all([
      lstat(skillMd).catch(() => null),
      lstat(pkgJson).catch(() => null),
    ]);
    expect(skillStat?.isFile()).toBe(true);
    expect(pkgStat?.isFile()).toBe(true);
  });

  // ── verify ───────────────────────────────────────────────────────────────────

  test("5. verify basic skill — exit 0", async () => {
    const { exitCode } = await run(
      ["verify", join(workDir, "my-e2e-skill")],
    );
    expect(exitCode).toBe(0);
  });

  test("6. verify basic skill — --json output is valid", async () => {
    const { exitCode, stdout } = await run([
      "verify",
      join(workDir, "my-e2e-skill"),
      "--json",
    ]);
    expect(exitCode).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(parsed.valid).toBe(true);
    expect(Array.isArray(parsed.issues)).toBe(true);
    expect(parsed.name).toBe("my-e2e-skill");
  });

  test("7. verify npm skill — exit 0", async () => {
    const { exitCode } = await run(
      ["verify", join(workDir, "my-npm-skill")],
    );
    expect(exitCode).toBe(0);
  });

  // ── doctor ───────────────────────────────────────────────────────────────────

  test("8. doctor --json — exit 0 in clean state", async () => {
    const { exitCode, stdout } = await run(["doctor", "--json"]);
    expect(exitCode).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(typeof parsed.ok).toBe("boolean");
    expect(Array.isArray(parsed.checks)).toBe(true);
    expect(parsed.checks.length).toBeGreaterThan(0);
  });

  test("9. doctor --json — checks have required fields", async () => {
    const { stdout } = await run(["doctor", "--json"]);
    const parsed = JSON.parse(stdout);
    for (const check of parsed.checks) {
      expect(typeof check.name).toBe("string");
      expect(["pass", "warn", "fail"]).toContain(check.status);
    }
  });

  test("10. doctor (interactive) — exit 0 in clean state", async () => {
    const { exitCode, stdout, stderr } = await run(["doctor"]);
    // Exit 0 means no failures (warnings are OK)
    expect(exitCode).toBe(0);
    // Should mention at least one check name
    expect(stdout + stderr).toMatch(/git|config|dirs|installed/i);
  });

  // ── completions ──────────────────────────────────────────────────────────────

  test("11. completions bash — prints non-empty script", async () => {
    const { exitCode, stdout } = await run(["completions", "bash"]);
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
    expect(stdout).toMatch(/skilltap/i);
  });

  test("12. completions zsh — prints non-empty script", async () => {
    const { exitCode, stdout } = await run(["completions", "zsh"]);
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
    expect(stdout).toMatch(/skilltap/i);
  });

  test("13. completions fish — prints non-empty script", async () => {
    const { exitCode, stdout } = await run(["completions", "fish"]);
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
    expect(stdout).toMatch(/skilltap/i);
  });

  test("14. completions unknown shell — exit 1", async () => {
    const { exitCode, stderr } = await run(["completions", "powershell"]);
    expect(exitCode).toBe(1);
    expect(stderr).toMatch(/unknown shell|valid values/i);
  });

  test("15. completions bash --install — writes file", async () => {
    const { exitCode, stdout, stderr } = await run([
      "completions",
      "bash",
      "--install",
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/wrote completions to/i);
  });
});
