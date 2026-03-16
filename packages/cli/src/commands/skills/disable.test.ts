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
import { $ } from "bun";
import { makeTmpDir, removeTmpDir, runSkilltap } from "@skilltap/test-utils";

setDefaultTimeout(45_000);

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

describe("skilltap skills disable/enable — full E2E roundtrip", () => {
  test("disable → enable roundtrip updates installed.json state correctly", async () => {
    // Note: the list display of disabled skills is skipped as a separate spec violation
    // (discoverSkills does not scan .disabled/). This test verifies the core disable→enable
    // state machine works correctly via CLI, using installed.json as ground truth.
    await seedGlobalSkill("roundtrip-skill");

    // Verify skill is active
    let loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills.find((s) => s.name === "roundtrip-skill")?.active).toBe(true);

    // Disable via CLI
    const disableResult = await runSkilltap(
      ["skills", "disable", "roundtrip-skill"],
      homeDir,
      configDir,
    );
    expect(disableResult.exitCode).toBe(0);
    expect(disableResult.stdout).toContain("roundtrip-skill");

    // Record should be inactive
    loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills.find((s) => s.name === "roundtrip-skill")?.active).toBe(false);

    // Enable via CLI
    const enableResult = await runSkilltap(
      ["skills", "enable", "roundtrip-skill"],
      homeDir,
      configDir,
    );
    expect(enableResult.exitCode).toBe(0);
    expect(enableResult.stdout).toContain("roundtrip-skill");

    // Record should be active again
    loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills.find((s) => s.name === "roundtrip-skill")?.active).toBe(true);
  });

  test("list shows 'disabled' after disable and 'managed' after enable", async () => {
    await seedGlobalSkill("roundtrip-skill");

    const listBefore = await runSkilltap(["skills"], homeDir, configDir);
    expect(listBefore.exitCode).toBe(0);
    expect(listBefore.stdout).toContain("roundtrip-skill");
    expect(listBefore.stdout.toLowerCase()).not.toContain("disabled");

    await runSkilltap(["skills", "disable", "roundtrip-skill"], homeDir, configDir);

    const listDisabled = await runSkilltap(["skills"], homeDir, configDir);
    expect(listDisabled.exitCode).toBe(0);
    expect(listDisabled.stdout).toContain("roundtrip-skill");
    expect(listDisabled.stdout.toLowerCase()).toContain("disabled");

    await runSkilltap(["skills", "enable", "roundtrip-skill"], homeDir, configDir);

    const listEnabled = await runSkilltap(["skills"], homeDir, configDir);
    expect(listEnabled.exitCode).toBe(0);
    expect(listEnabled.stdout).toContain("roundtrip-skill");
    expect(listEnabled.stdout.toLowerCase()).not.toContain("disabled");
  });
});

describe("skilltap skills disable — project scope", () => {
  test("--project flag disables a project-scoped skill", async () => {
    const projectRoot = await makeTmpDir();
    try {
      await $`git -C ${projectRoot} init`.quiet();

      const skillDir = join(projectRoot, ".agents", "skills", "proj-skill");
      await mkdir(skillDir, { recursive: true });
      await Bun.write(join(skillDir, "SKILL.md"), "---\nname: proj-skill\n---\n");

      await saveInstalled(
        {
          version: 1,
          skills: [
            {
              name: "proj-skill",
              description: "",
              repo: "https://github.com/example/repo",
              ref: "main",
              sha: null,
              scope: "project",
              path: null,
              tap: null,
              also: [],
              installedAt: NOW,
              updatedAt: NOW,
              active: true,
            },
          ],
        },
        projectRoot,
      );

      const { exitCode, stdout } = await runSkilltap(
        ["skills", "disable", "proj-skill", "--project"],
        homeDir,
        configDir,
        projectRoot,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("proj-skill");

      // Verify project installed.json updated
      const loaded = await loadInstalled(projectRoot);
      expect(loaded.ok).toBe(true);
      if (!loaded.ok) return;
      const record = loaded.value.skills.find((s) => s.name === "proj-skill");
      expect(record?.active).toBe(false);

      // Verify files moved to .disabled/ within project
      const disabledDir = join(projectRoot, ".agents", "skills", ".disabled", "proj-skill");
      const { lstat } = await import("node:fs/promises");
      expect(await lstat(disabledDir).then((s) => s.isDirectory())).toBe(true);
      expect(await lstat(skillDir).catch(() => null)).toBeNull();
    } finally {
      await removeTmpDir(projectRoot);
    }
  });
});
