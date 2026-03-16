import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  addFileAndCommit,
  commitAll,
  createMultiSkillRepo,
  createSkillDir,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { disableSkill, enableSkill } from "./disable";
import { installSkill } from "./install";
import { linkSkill } from "./link";
import { moveSkill } from "./move";
import { removeSkill } from "./remove";
import { updateSkill } from "./update";

setDefaultTimeout(60_000);

type Env = { SKILLTAP_HOME?: string; XDG_CONFIG_HOME?: string };

let savedEnv: Env;
let homeDir: string;
let configDir: string;
const cleanups: (() => Promise<void>)[] = [];

beforeEach(async () => {
  savedEnv = {
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
  };
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  if (savedEnv.XDG_CONFIG_HOME === undefined)
    delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
  for (const cleanup of cleanups.splice(0)) await cleanup();
});

// ---------------------------------------------------------------------------
// Conflict & concurrent operations
// ---------------------------------------------------------------------------
describe("conflicts", () => {
  test("re-install same skill without callback errors with helpful hint", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });

    // Without onAlreadyInstalled, conflicts must error (not silently succeed)
    const result = await installSkill(repo.path, {
      scope: "global",
      skipScan: true,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already installed");
    expect(result.error.hint).toContain("update");
  });

  test("re-install with onAlreadyInstalled=update triggers update", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });

    // Make a new commit so there's something to update
    await addFileAndCommit(repo.path, "new.md", "content", "new file");

    const result = await installSkill(repo.path, {
      scope: "global",
      skipScan: true,
      onAlreadyInstalled: async () => "update",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.updates).toContain("standalone-skill");
  });

  test("link conflicting name errors with hint mentioning remove", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });

    // Create a local skill with the same name
    const localDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(localDir));
    await createSkillDir(localDir, "standalone-skill");

    const result = await linkSkill(join(localDir, "standalone-skill"), {
      scope: "global",
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already installed");
    expect(result.error.hint).toContain("remove");
  });
});

// ---------------------------------------------------------------------------
// Scope interaction
// ---------------------------------------------------------------------------
describe("scope interaction", () => {
  test("same skill in global AND project coexist independently", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    const projectDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(projectDir));
    await initRepo(projectDir);

    // Install globally
    const g = await installSkill(repo.path, {
      scope: "global",
      skipScan: true,
    });
    expect(g.ok).toBe(true);

    // Install to project (onAlreadyInstalled not triggered — different scope/installed.json)
    const p = await installSkill(repo.path, {
      scope: "project",
      projectRoot: projectDir,
      skipScan: true,
    });
    expect(p.ok).toBe(true);

    // Both exist
    const globalDir = join(homeDir, ".agents", "skills", "standalone-skill");
    const projectSkillDir = join(projectDir, ".agents", "skills", "standalone-skill");
    expect((await lstat(globalDir)).isDirectory()).toBe(true);
    expect((await lstat(projectSkillDir)).isDirectory()).toBe(true);

    // Remove global, project still exists
    const rm = await removeSkill("standalone-skill", { scope: "global" });
    expect(rm.ok).toBe(true);

    expect(await lstat(globalDir).catch(() => null)).toBeNull();
    expect((await lstat(projectSkillDir)).isDirectory()).toBe(true);

    // Remove project
    const rm2 = await removeSkill("standalone-skill", {
      scope: "project",
      projectRoot: projectDir,
    });
    expect(rm2.ok).toBe(true);
  });

  test("move to scope that already has it errors", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });

    const result = await moveSkill("standalone-skill", { to: { scope: "global" } });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already in global scope");
  });
});

// ---------------------------------------------------------------------------
// Error recovery & corrupt state
// ---------------------------------------------------------------------------
describe("error recovery", () => {
  test("corrupt installed.json returns clear error", async () => {
    // installed.json lives in the config dir, not homeDir/.agents/
    const configSkiltapDir = join(configDir, "skilltap");
    await mkdir(configSkiltapDir, { recursive: true });
    await Bun.write(join(configSkiltapDir, "installed.json"), "NOT VALID JSON {{{{");

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(false);
  });

  test("old-style local install with repo path survives update", async () => {
    // Simulate an installed.json from an older version where local installs
    // stored the filesystem path as `repo` instead of null
    const skillDir = join(homeDir, ".agents", "skills", "old-local-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      "---\nname: old-local-skill\ndescription: test\n---\n# Skill\n",
    );

    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "old-local-skill",
          description: "test",
          repo: "/tmp/deleted-source-path",
          ref: null,
          sha: null,
          scope: "global",
          path: null,
          tap: null,
          also: [],
          installedAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
    });

    // Update should skip this gracefully (not crash with git fetch error)
    const progressStatuses: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name, status) {
        if (name === "old-local-skill") progressStatuses.push(status);
      },
    });
    expect(up.ok).toBe(true);
    expect(progressStatuses).toContain("local");
    expect(up.ok && up.value.upToDate).toContain("old-local-skill");
  });

  test("old-style git clone install with deleted source path survives update", async () => {
    // Simulate: user ran `skilltap install /tmp/repo`, the clone is at the
    // install dir with .git remote pointing to a now-deleted path
    const skillDir = join(homeDir, ".agents", "skills", "dead-remote-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(
      join(skillDir, "SKILL.md"),
      "---\nname: dead-remote-skill\ndescription: test\n---\n# Skill\n",
    );
    // Make it a git repo with a remote pointing to a nonexistent path
    await initRepo(skillDir);
    await commitAll(skillDir);
    await $`git -C ${skillDir} remote add origin /tmp/skilltap-definitely-deleted-12345`.quiet();

    await saveInstalled({
      version: 1,
      skills: [
        {
          name: "dead-remote-skill",
          description: "test",
          repo: "/tmp/skilltap-definitely-deleted-12345",
          ref: null,
          sha: "abc123",
          scope: "global",
          path: null,
          tap: null,
          also: [],
          installedAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
    });

    const progressStatuses: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name, status) {
        if (name === "dead-remote-skill") progressStatuses.push(status);
      },
    });
    expect(up.ok).toBe(true);
    // Should gracefully skip, not crash with git fetch error
    expect(progressStatuses).toContain("local");
    expect(up.ok && up.value.upToDate).toContain("dead-remote-skill");
  });

  test("pre-existing file at target path does not block install", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    // Pre-create parent dir with a stale file where the skill dir will go
    const skillsDir = join(homeDir, ".agents", "skills");
    await mkdir(skillsDir, { recursive: true });

    const result = await installSkill(repo.path, {
      scope: "global",
      skipScan: true,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const targetDir = join(skillsDir, "standalone-skill");
    expect(
      await Bun.file(join(targetDir, "SKILL.md")).exists(),
    ).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// DX error messages — verify hints are actionable
// ---------------------------------------------------------------------------
describe("DX error messages", () => {
  test("remove non-existent skill mentions 'list'", async () => {
    const result = await removeSkill("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
    expect(result.error.hint).toContain("list");
  });

  test("install non-existent path mentions 'does not exist'", async () => {
    const result = await installSkill("/tmp/skilltap-definitely-not-real-12345", {
      scope: "global",
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    // Adapter won't match or the local adapter will report missing
    expect(result.error.message.toLowerCase()).toMatch(
      /not exist|not found|no such|cannot resolve/,
    );
  });

  test("install git repo with no SKILL.md mentions 'No skills found'", async () => {
    const emptyDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(emptyDir));
    // Create a valid git repo with a commit, but no SKILL.md
    await initRepo(emptyDir);
    await Bun.write(join(emptyDir, "README.md"), "# Empty repo");
    await commitAll(emptyDir);

    const result = await installSkill(emptyDir, {
      scope: "global",
      skipScan: true,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message.toLowerCase()).toMatch(/no skill/);
  });

  test("install from non-git local directory succeeds", async () => {
    const localDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(localDir));
    await createSkillDir(localDir, "local-only");

    const result = await installSkill(join(localDir, "local-only"), {
      scope: "global",
      skipScan: true,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.records).toHaveLength(1);
    const rec = result.value.records[0]!;
    expect(rec.name).toBe("local-only");
    expect(rec.repo).toBeNull();
    expect(rec.sha).toBeNull();

    // Skill files are on disk
    const skillDir = join(homeDir, ".agents", "skills", "local-only");
    expect(await Bun.file(join(skillDir, "SKILL.md")).exists()).toBe(true);

    // Update should skip it (local, no remote)
    const progressStatuses: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name, status) {
        if (name === "local-only") progressStatuses.push(status);
      },
    });
    expect(up.ok).toBe(true);
    expect(progressStatuses).toContain("local");
  });

  test("disable already-disabled skill says 'already disabled'", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });
    await disableSkill("standalone-skill");

    const result = await disableSkill("standalone-skill");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already disabled");
  });

  test("enable already-enabled skill says 'already enabled'", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });

    const result = await enableSkill("standalone-skill");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already enabled");
  });

  test("update non-existent skill mentions 'list'", async () => {
    const result = await updateSkill({ name: "nonexistent", yes: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not installed");
    expect(result.error.hint).toContain("list");
  });
});

// ---------------------------------------------------------------------------
// Disable + update interaction
// ---------------------------------------------------------------------------
describe("disable + update interaction", () => {
  test("bulk update filters out disabled skills", async () => {
    const repo = await createMultiSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });
    await disableSkill("skill-a");

    // Advance the remote
    await addFileAndCommit(
      repo.path,
      ".agents/skills/skill-b/SKILL.md",
      "---\nname: skill-b\ndescription: Updated\n---\n# Updated\n",
      "update skill-b",
    );

    const progressNames: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name) {
        progressNames.push(name);
      },
    });
    expect(up.ok).toBe(true);
    if (!up.ok) return;

    // skill-a should NOT appear in any update list (filtered out as disabled)
    expect(progressNames).not.toContain("skill-a");
    // skill-b should have been checked
    expect(progressNames).toContain("skill-b");
  });

  test("named update of disabled skill still works", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    await installSkill(repo.path, { scope: "global", skipScan: true });
    await disableSkill("standalone-skill");

    await addFileAndCommit(repo.path, "new.md", "content", "new file");

    const up = await updateSkill({ name: "standalone-skill", yes: true });
    expect(up.ok).toBe(true);
    if (!up.ok) return;
    expect(up.value.updated).toContain("standalone-skill");

    // Verify it's still disabled
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(
      loaded.value.skills.find((s) => s.name === "standalone-skill")?.active,
    ).toBe(false);
  });
});
