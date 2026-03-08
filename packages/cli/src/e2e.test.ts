/**
 * End-to-end lifecycle test: exercises the full skilltap CLI as a subprocess.
 * Tests run sequentially and share state via homeDir/configDir.
 */
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";

const CLI_ENTRY = `${import.meta.dir}/../src/index.ts`;

let homeDir: string;
let configDir: string;
let skillRepo: { path: string; cleanup: () => Promise<void> };
let tapRepo: { path: string; cleanup: () => Promise<void> };

async function run(
  args: string[],
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["bun", "run", "--bun", CLI_ENTRY, ...args], {
    cwd: homeDir,
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

async function createTapRepo(skillRepoPath: string) {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name: "e2e-tap",
    description: "E2E test tap",
    skills: [
      {
        name: "standalone-skill",
        description: "A standalone test skill",
        repo: skillRepoPath,
        tags: ["test", "e2e"],
      },
    ],
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  skillRepo = await createStandaloneSkillRepo();
  tapRepo = await createTapRepo(skillRepo.path);
});

afterAll(async () => {
  await skillRepo.cleanup();
  await tapRepo.cleanup();
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

describe("E2E lifecycle", () => {
  test("1. list — empty state", async () => {
    const { exitCode, stdout } = await run(["list"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No skills installed");
    expect(stdout).toContain("to get started");
  });

  test("2. tap add", async () => {
    const { exitCode, stdout, stderr } = await run([
      "tap",
      "add",
      "e2e-tap",
      tapRepo.path,
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/added|e2e-tap/i);
  });

  test("3. tap list — shows e2e-tap", async () => {
    const { exitCode, stdout } = await run(["tap", "list"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("e2e-tap");
  });

  test("4. find — shows skills from tap", async () => {
    const { exitCode, stdout } = await run(["find"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
  });

  test("5. install — installs skill", async () => {
    const { exitCode, stdout, stderr } = await run([
      "install",
      skillRepo.path,
      "--yes",
      "--global",
      "--skip-scan",
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/installed|standalone-skill/i);
  });

  test("6. list — shows installed skill", async () => {
    const { exitCode, stdout } = await run(["list"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
  });

  test("7. info — shows skill details", async () => {
    const { exitCode, stdout } = await run(["info", "standalone-skill"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("standalone-skill");
    expect(stdout).toContain("global");
  });

  test("8. list --json — valid JSON array", async () => {
    const { exitCode, stdout } = await run(["list", "--json"]);
    expect(exitCode).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed.length).toBeGreaterThan(0);
    expect(parsed[0].name).toBe("standalone-skill");
  });

  test("9. update --yes — reports up to date", async () => {
    const { exitCode, stdout, stderr } = await run([
      "update",
      "standalone-skill",
      "--yes",
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/up.to.date|already|updated/i);
  });

  test("10. remove — removes skill", async () => {
    const { exitCode, stdout, stderr } = await run([
      "remove",
      "standalone-skill",
      "--yes",
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/removed|standalone-skill/i);
  });

  test("11. list — empty again", async () => {
    const { exitCode, stdout } = await run(["list"]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("No skills installed");
  });

  test("12. tap remove — removes tap", async () => {
    const { exitCode, stdout, stderr } = await run([
      "tap",
      "remove",
      "e2e-tap",
      "--yes",
    ]);
    expect(exitCode).toBe(0);
    expect(stdout + stderr).toMatch(/removed|e2e-tap/i);
  });
});
