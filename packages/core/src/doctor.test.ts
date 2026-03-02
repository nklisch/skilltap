import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, symlink, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { runDoctor } from "./doctor";

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

// ─── Git check ────────────────────────────────────────────────────────────────

describe("checkGit", () => {
  test("passes when git is installed", async () => {
    const result = await runDoctor();
    const gitCheck = result.checks.find((c) => c.name === "git")!;
    // git is always available in CI, so this should pass
    expect(gitCheck.status).toBeOneOf(["pass", "warn"]);
    expect(gitCheck.detail).toContain("git");
  });
});

// ─── Config check ─────────────────────────────────────────────────────────────

describe("checkConfig", () => {
  test("warns when config.toml is missing", async () => {
    const result = await runDoctor();
    const configCheck = result.checks.find((c) => c.name === "config")!;
    // On a fresh configDir, no config.toml exists
    expect(configCheck.status).toBeOneOf(["pass", "warn"]);
  });

  test("passes when config.toml is valid", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "config.toml"),
      '[defaults]\nalso = []\nyes = false\nscope = ""\n[security]\nscan = "static"\non_warn = "prompt"\nrequire_scan = false\nagent = ""\nthreshold = 5\nmax_size = 51200\nollama_model = ""\n["agent-mode"]\nenabled = false\nscope = "project"\n',
    );

    const result = await runDoctor();
    const configCheck = result.checks.find((c) => c.name === "config")!;
    expect(configCheck.status).toBe("pass");
  });

  test("fails when config.toml has invalid TOML", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(join(skilltapDir, "config.toml"), "not valid toml ===\n");

    const result = await runDoctor();
    const configCheck = result.checks.find((c) => c.name === "config")!;
    expect(configCheck.status).toBe("fail");
    expect(configCheck.issues?.[0]?.message).toContain("invalid TOML");
  });
});

// ─── Dirs check ───────────────────────────────────────────────────────────────

describe("checkDirs", () => {
  test("warns when required directories are missing", async () => {
    const result = await runDoctor();
    const dirsCheck = result.checks.find((c) => c.name === "dirs")!;
    // On fresh tempDir, skilltap dirs don't exist yet
    expect(dirsCheck.status).toBeOneOf(["pass", "warn"]);
  });

  test("passes when all dirs exist", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(join(skilltapDir, "cache"), { recursive: true });
    await mkdir(join(skilltapDir, "taps"), { recursive: true });
    await mkdir(join(homeDir, ".agents", "skills"), { recursive: true });

    const result = await runDoctor();
    const dirsCheck = result.checks.find((c) => c.name === "dirs")!;
    expect(dirsCheck.status).toBe("pass");
  });

  test("--fix creates missing dirs", async () => {
    const result = await runDoctor({ fix: true });
    const dirsCheck = result.checks.find((c) => c.name === "dirs")!;
    // After fix, all fixable issues should be fixed
    const unfixed = dirsCheck.issues?.filter((i) => i.fixable && !i.fixed) ?? [];
    expect(unfixed).toHaveLength(0);
  });
});

// ─── installed.json check ─────────────────────────────────────────────────────

describe("checkInstalled", () => {
  test("passes with no installed.json (0 skills)", async () => {
    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "installed")!;
    expect(check.status).toBe("pass");
    expect(check.detail).toContain("0 skills");
  });

  test("passes with valid installed.json", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify({ version: 1, skills: [] }, null, 2),
    );

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "installed")!;
    expect(check.status).toBe("pass");
  });

  test("fails with corrupt installed.json", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(join(skilltapDir, "installed.json"), "not json {{{}}}");

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "installed")!;
    expect(check.status).toBe("fail");
    expect(check.issues?.[0]?.message).toContain("corrupt");
  });

  test("--fix repairs corrupt installed.json", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    const installedFile = join(skilltapDir, "installed.json");
    await writeFile(installedFile, "not json {{{}}}");

    const result = await runDoctor({ fix: true });
    const check = result.checks.find((c) => c.name === "installed")!;
    expect(check.issues?.[0]?.fixed).toBe(true);

    // Verify the file is now valid JSON
    const repaired = await Bun.file(installedFile).json();
    expect(repaired).toEqual({ version: 1, skills: [] });
  });
});

// ─── Skills integrity check ───────────────────────────────────────────────────

describe("checkSkills", () => {
  test("passes with no skills", async () => {
    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "skills")!;
    expect(check.status).toBe("pass");
  });

  test("warns on orphan record (skill in installed.json but not on disk)", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "missing-skill",
              description: "",
              repo: "https://github.com/example/missing-skill",
              ref: null,
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: [],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "skills")!;
    expect(check.status).toBe("warn");
    expect(check.issues?.some((i) => i.message.includes("missing-skill"))).toBe(
      true,
    );
  });

  test("warns on orphan directory (dir on disk but not in installed.json)", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify({ version: 1, skills: [] }, null, 2),
    );

    // Create orphan dir
    const orphanDir = join(homeDir, ".agents", "skills", "orphan-skill");
    await mkdir(orphanDir, { recursive: true });

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "skills")!;
    expect(check.status).toBe("warn");
    expect(
      check.issues?.some((i) => i.message.includes("orphan-skill")),
    ).toBe(true);
    // Orphan dirs are not fixable
    expect(
      check.issues?.find((i) => i.message.includes("orphan-skill"))?.fixable,
    ).toBe(false);
  });

  test("--fix removes orphan records from installed.json", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    const installedFile = join(skilltapDir, "installed.json");
    await writeFile(
      installedFile,
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "missing-skill",
              description: "",
              repo: "https://github.com/example/missing",
              ref: null,
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: [],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    await runDoctor({ fix: true });

    // The orphan record should be removed
    const repaired = await Bun.file(installedFile).json();
    expect(repaired.skills).toHaveLength(0);
  });
});

// ─── Symlinks check ───────────────────────────────────────────────────────────

describe("checkSymlinks", () => {
  test("passes when no symlinks are configured", async () => {
    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "symlinks")!;
    expect(check.status).toBe("pass");
    expect(check.detail).toContain("0 symlinks");
  });

  test("warns on missing agent symlink", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });

    // Create a real skill dir
    const skillDir = join(homeDir, ".agents", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });

    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "my-skill",
              description: "",
              repo: "https://github.com/example/my-skill",
              ref: null,
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: ["claude-code"],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "symlinks")!;
    expect(check.status).toBe("warn");
    expect(check.issues?.some((i) => i.message.includes("my-skill"))).toBe(
      true,
    );
  });

  test("--fix recreates missing symlinks", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });

    const skillDir = join(homeDir, ".agents", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });

    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "my-skill",
              description: "",
              repo: "https://github.com/example/my-skill",
              ref: null,
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: ["claude-code"],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    const result = await runDoctor({ fix: true });
    const check = result.checks.find((c) => c.name === "symlinks")!;
    const issue = check.issues?.find((i) => i.message.includes("my-skill"));
    expect(issue?.fixed).toBe(true);

    // Verify symlink was created
    const linkPath = join(homeDir, ".claude", "skills", "my-skill");
    const { lstat: lstatFn } = await import("node:fs/promises");
    const stat = await lstatFn(linkPath);
    expect(stat.isSymbolicLink()).toBe(true);
  });

  test("warns on wrong symlink target", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });

    const skillDir = join(homeDir, ".agents", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });

    // Create symlink pointing to wrong target
    const linkDir = join(homeDir, ".claude", "skills");
    await mkdir(linkDir, { recursive: true });
    const wrongTarget = join(homeDir, "wrong-target");
    await mkdir(wrongTarget, { recursive: true });
    await symlink(wrongTarget, join(linkDir, "my-skill"), "dir");

    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "my-skill",
              description: "",
              repo: "https://github.com/example/my-skill",
              ref: null,
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: ["claude-code"],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "symlinks")!;
    expect(check.status).toBe("warn");
    expect(
      check.issues?.some((i) => i.message.includes("wrong target")),
    ).toBe(true);
  });
});

// ─── Taps check ───────────────────────────────────────────────────────────────

describe("checkTaps", () => {
  test("passes with no taps configured", async () => {
    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "taps")!;
    expect(check.status).toBe("pass");
    expect(check.detail).toContain("0 configured");
  });
});

// ─── Agents check ─────────────────────────────────────────────────────────────

describe("checkAgents", () => {
  test("passes (no configured agent to verify)", async () => {
    const result = await runDoctor();
    const check = result.checks.find((c) => c.name === "agents")!;
    expect(check.status).toBe("pass");
  });
});

// ─── Overall result ───────────────────────────────────────────────────────────

describe("runDoctor", () => {
  test("ok is true when no failures", async () => {
    const result = await runDoctor();
    // On a fresh env, no hard failures expected
    const hasFailures = result.checks.some((c) => c.status === "fail");
    expect(result.ok).toBe(!hasFailures);
  });

  test("ok is false when there is a failure", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(join(skilltapDir, "config.toml"), "not valid toml ===\n");

    const result = await runDoctor();
    expect(result.ok).toBe(false);
  });

  test("onCheck callback is called for each check", async () => {
    const called: string[] = [];
    await runDoctor({
      onCheck: (check) => {
        called.push(check.name);
      },
    });

    expect(called).toContain("git");
    expect(called).toContain("config");
    expect(called).toContain("dirs");
    expect(called).toContain("installed");
    expect(called).toContain("skills");
    expect(called).toContain("symlinks");
    expect(called).toContain("taps");
    expect(called).toContain("agents");
  });

  test("npm check is skipped when no npm skills installed", async () => {
    const called: string[] = [];
    await runDoctor({
      onCheck: (check) => {
        called.push(check.name);
      },
    });
    expect(called).not.toContain("npm");
  });

  test("npm check runs when npm skills are installed", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify(
        {
          version: 1,
          skills: [
            {
              name: "npm-skill",
              description: "",
              repo: "npm:@example/npm-skill",
              ref: "1.0.0",
              sha: null,
              scope: "global",
              path: null,
              tap: null,
              also: [],
              installedAt: "2024-01-01T00:00:00.000Z",
              updatedAt: "2024-01-01T00:00:00.000Z",
            },
          ],
        },
        null,
        2,
      ),
    );

    const called: string[] = [];
    await runDoctor({
      onCheck: (check) => {
        called.push(check.name);
      },
    });
    expect(called).toContain("npm");
  });
});
