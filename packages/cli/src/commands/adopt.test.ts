import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, mkdir, readlink } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir, runSkilltap } from "@skilltap/test-utils";

setDefaultTimeout(60_000);

let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

/** Creates a minimal skill dir with a valid SKILL.md. */
async function createSkillDir(
  baseDir: string,
  name: string,
): Promise<string> {
  const skillDir = join(baseDir, name);
  await mkdir(skillDir, { recursive: true });
  await Bun.write(
    join(skillDir, "SKILL.md"),
    `---\nname: ${name}\ndescription: A test skill for adopt CLI\n---\n# ${name}\nContent.\n`,
  );
  return skillDir;
}

// ─── Path mode ────────────────────────────────────────────────────────────────

describe("adopt — path mode", () => {
  test("adopts external skill via path (track-in-place default): creates symlink in .agents/skills/", async () => {
    const externalDir = join(homeDir, "external");
    const skillPath = await createSkillDir(externalDir, "my-ext-skill");

    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt", skillPath, "--scope", "global", "--skip-scan"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);
    expect(stdout + stderr).toContain("Adopted skill from");
    expect(stdout + stderr).toContain("my-ext-skill");

    // Symlink should exist in .agents/skills/
    const symlinkPath = join(homeDir, ".agents", "skills", "my-ext-skill");
    const stat = await lstat(symlinkPath).catch(() => null);
    expect(stat?.isSymbolicLink()).toBe(true);

    // Symlink points back to original path
    const target = await readlink(symlinkPath);
    expect(target).toBe(skillPath);

    // Original dir still intact
    const origStat = await lstat(skillPath).catch(() => null);
    expect(origStat?.isDirectory()).toBe(true);
  });

  test("--move physically moves the dir and creates back-symlink at original location", async () => {
    const externalDir = join(homeDir, "external2");
    const skillPath = await createSkillDir(externalDir, "move-skill");

    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt", skillPath, "--scope", "global", "--skip-scan", "--move"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);
    expect(stdout + stderr).toContain("move-skill");

    // Dir moved to canonical location
    const targetPath = join(homeDir, ".agents", "skills", "move-skill");
    const targetStat = await lstat(targetPath).catch(() => null);
    expect(targetStat?.isDirectory()).toBe(true);

    // Back-symlink at original path
    const origStat = await lstat(skillPath).catch(() => null);
    expect(origStat?.isSymbolicLink()).toBe(true);
  });

  test("invalid path (no SKILL.md) errors with clear message", async () => {
    const emptyDir = join(homeDir, "empty-dir");
    await mkdir(emptyDir, { recursive: true });

    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt", emptyDir, "--scope", "global", "--skip-scan"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(1);
    expect(stdout + stderr).toContain("SKILL.md");
  });

  test("--skip-scan bypasses security scan and succeeds", async () => {
    const externalDir = join(homeDir, "external3");
    const skillPath = await createSkillDir(externalDir, "scan-skip-skill");

    const { exitCode } = await runSkilltap(
      ["adopt", skillPath, "--scope", "global", "--skip-scan"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);
  });
});

// ─── Name mode ────────────────────────────────────────────────────────────────

describe("adopt — name mode", () => {
  test("adopts unmanaged skill by name (discovers from .claude/skills/)", async () => {
    // Create skill in .claude/skills so it appears as unmanaged
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await createSkillDir(claudeSkillsDir, "named-skill");

    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt", "named-skill", "--scope", "global", "--skip-scan"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(0);
    expect(stdout + stderr).toContain("named-skill");
  });

  test("errors with clear message when name not found", async () => {
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt", "nonexistent-skill", "--scope", "global"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(1);
    const output = stdout + stderr;
    expect(output).toContain("nonexistent-skill");
  });
});

// ─── Non-interactive mode ─────────────────────────────────────────────────────

describe("adopt — non-interactive", () => {
  test("bare adopt with no target (non-TTY stdin) errors with usage hint", async () => {
    // runSkilltap uses Bun.spawn with piped stdin — isTTY will be false
    const { exitCode, stdout, stderr } = await runSkilltap(
      ["adopt"],
      homeDir,
      configDir,
    );

    expect(exitCode).toBe(1);
    const output = stdout + stderr;
    // Should show usage hint
    expect(output).toContain("adopt requires a target");
    expect(output).toContain("Usage:");
  });
});
