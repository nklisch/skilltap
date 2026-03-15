import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { join } from "node:path";
import { mkdir } from "node:fs/promises";
import { loadInstalled, saveInstalled } from "@skilltap/core";
import { makeTmpDir, removeTmpDir, runSkilltap } from "@skilltap/test-utils";

setDefaultTimeout(30_000);

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

const NOW = "2026-01-01T00:00:00.000Z";

async function seedGlobalSkill(name: string) {
  const skillDir = join(homeDir, ".agents", "skills", name);
  await mkdir(skillDir, { recursive: true });
  await Bun.write(join(skillDir, "SKILL.md"), `---\nname: ${name}\n---\n`);
  await saveInstalled({
    version: 1,
    skills: [
      {
        name,
        description: "",
        repo: "https://github.com/example/repo",
        ref: "main",
        sha: null,
        scope: "global",
        path: null,
        tap: null,
        also: [],
        installedAt: NOW,
        updatedAt: NOW,
        active: true,
      },
    ],
  });
  return skillDir;
}

describe("skilltap skills disable", () => {
  test("disables a skill and prints success", async () => {
    await seedGlobalSkill("my-skill");

    const { exitCode, stdout } = await runSkilltap(
      ["skills", "disable", "my-skill"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("my-skill");

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    const record = loaded.value.skills.find((s) => s.name === "my-skill");
    expect(record?.active).toBe(false);
  });

  test("errors on unknown skill with exit code 1", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["skills", "disable", "nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});

describe("skilltap skills enable", () => {
  test("enables a disabled skill and prints success", async () => {
    await seedGlobalSkill("my-skill");

    // First disable it
    await runSkilltap(["skills", "disable", "my-skill"], homeDir, configDir);

    const { exitCode, stdout } = await runSkilltap(
      ["skills", "enable", "my-skill"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("my-skill");

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    const record = loaded.value.skills.find((s) => s.name === "my-skill");
    expect(record?.active).toBe(true);
  });

  test("errors on unknown skill with exit code 1", async () => {
    const { exitCode, stderr } = await runSkilltap(
      ["skills", "enable", "nonexistent"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("not installed");
  });
});
