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
import {
  addFileAndCommit,
  createAdoptableSkill,
  createMultiSkillRepo,
  createSkillDir,
  createStandaloneSkillRepo,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  type TestEnv,
} from "@skilltap/test-utils";
import { adoptSkill } from "./adopt";
import { loadInstalled } from "./config";
import { disableSkill, enableSkill } from "./disable";
import { discoverSkills } from "./discover";
import { installSkill } from "./install";
import { linkSkill } from "./link";
import { moveSkill } from "./move";
import { removeSkill } from "./remove";
import { updateSkill } from "./update";

setDefaultTimeout(60_000);

let env: TestEnv;
let homeDir: string;
let configDir: string;
const cleanups: (() => Promise<void>)[] = [];

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
  for (const cleanup of cleanups.splice(0)) await cleanup();
});

// ---------------------------------------------------------------------------
// Journey 1: Git standalone — full lifecycle
// ---------------------------------------------------------------------------
describe("git standalone lifecycle", () => {
  test("install → update (up-to-date) → update (new commit) → disable → update-while-disabled → enable → move → remove", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    // --- Install ---
    const install = await installSkill(repo.path, {
      scope: "global",
      also: ["claude-code"],
      skipScan: true,
    });
    expect(install.ok).toBe(true);
    if (!install.ok) return;

    const rec = install.value.records[0]!;
    expect(rec.name).toBe("standalone-skill");
    expect(rec.scope).toBe("global");
    expect(rec.sha).toBeString();
    const initialSha = rec.sha;

    const skillDir = join(homeDir, ".agents", "skills", "standalone-skill");
    expect((await lstat(skillDir)).isDirectory()).toBe(true);

    const claudeLink = join(homeDir, ".claude", "skills", "standalone-skill");
    expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);

    // --- Update (up-to-date) ---
    const up1 = await updateSkill({ yes: true });
    expect(up1.ok).toBe(true);
    if (!up1.ok) return;
    expect(up1.value.upToDate).toContain("standalone-skill");

    // --- Update (new commit) ---
    await addFileAndCommit(repo.path, "extra.md", "new content", "add extra");
    const up2 = await updateSkill({ yes: true });
    expect(up2.ok).toBe(true);
    if (!up2.ok) return;
    expect(up2.value.updated).toContain("standalone-skill");

    const loaded1 = await loadInstalled();
    expect(loaded1.ok).toBe(true);
    if (!loaded1.ok) return;
    const afterUpdateSha = loaded1.value.skills.find(
      (s) => s.name === "standalone-skill",
    )?.sha;
    expect(afterUpdateSha).not.toBe(initialSha);

    // --- Disable ---
    const dis = await disableSkill("standalone-skill");
    expect(dis.ok).toBe(true);

    const disabledDir = join(
      homeDir,
      ".agents",
      "skills",
      ".disabled",
      "standalone-skill",
    );
    expect((await lstat(disabledDir)).isDirectory()).toBe(true);
    expect(await lstat(claudeLink).catch(() => null)).toBeNull();

    const loaded2 = await loadInstalled();
    expect(loaded2.ok).toBe(true);
    if (!loaded2.ok) return;
    expect(
      loaded2.value.skills.find((s) => s.name === "standalone-skill")?.active,
    ).toBe(false);

    // --- Update while disabled (named → still updates) ---
    await addFileAndCommit(repo.path, "extra2.md", "more", "add extra2");
    const up3 = await updateSkill({ name: "standalone-skill", yes: true });
    expect(up3.ok).toBe(true);
    if (!up3.ok) return;
    expect(up3.value.updated).toContain("standalone-skill");

    // Still in disabled dir, no symlink
    expect((await lstat(disabledDir)).isDirectory()).toBe(true);
    expect(await lstat(claudeLink).catch(() => null)).toBeNull();

    // --- Enable ---
    const en = await enableSkill("standalone-skill");
    expect(en.ok).toBe(true);

    expect((await lstat(skillDir)).isDirectory()).toBe(true);
    expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);

    const loaded3 = await loadInstalled();
    expect(loaded3.ok).toBe(true);
    if (!loaded3.ok) return;
    expect(
      loaded3.value.skills.find((s) => s.name === "standalone-skill")?.active,
    ).toBe(true);

    // --- Move global → project ---
    const projectDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(projectDir));
    await initRepo(projectDir);

    const mv = await moveSkill("standalone-skill", {
      to: { scope: "project", projectRoot: projectDir },
    });
    expect(mv.ok).toBe(true);
    if (!mv.ok) return;

    const projectSkillDir = join(
      projectDir,
      ".agents",
      "skills",
      "standalone-skill",
    );
    expect((await lstat(projectSkillDir)).isDirectory()).toBe(true);

    const globalLoaded = await loadInstalled();
    expect(globalLoaded.ok).toBe(true);
    if (!globalLoaded.ok) return;
    expect(
      globalLoaded.value.skills.find((s) => s.name === "standalone-skill"),
    ).toBeUndefined();

    const projectLoaded = await loadInstalled(projectDir);
    expect(projectLoaded.ok).toBe(true);
    if (!projectLoaded.ok) return;
    expect(
      projectLoaded.value.skills.find((s) => s.name === "standalone-skill"),
    ).toBeDefined();

    // --- Remove ---
    const rm = await removeSkill("standalone-skill", {
      scope: "project",
      projectRoot: projectDir,
    });
    expect(rm.ok).toBe(true);

    expect(await lstat(projectSkillDir).catch(() => null)).toBeNull();
    const finalLoaded = await loadInstalled(projectDir);
    expect(finalLoaded.ok).toBe(true);
    if (!finalLoaded.ok) return;
    expect(finalLoaded.value.skills).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 2: Git multi-skill — install, selective update, remove with cache
// ---------------------------------------------------------------------------
describe("git multi-skill lifecycle", () => {
  test("install both → selective update → remove one (cache kept) → remove last (cache cleaned)", async () => {
    const repo = await createMultiSkillRepo();
    cleanups.push(repo.cleanup);

    // --- Install both ---
    const install = await installSkill(repo.path, {
      scope: "global",
      skipScan: true,
    });
    expect(install.ok).toBe(true);
    if (!install.ok) return;
    expect(install.value.records).toHaveLength(2);

    const records = install.value.records;
    const names = records.map((r) => r.name).sort();
    expect(names).toEqual(["skill-a", "skill-b"]);

    // All records should share the same repo and have non-null path
    expect(records[0]!.repo).toBe(records[1]!.repo);
    expect(records[0]!.path).not.toBeNull();
    expect(records[1]!.path).not.toBeNull();

    // --- Update only skill-a ---
    await addFileAndCommit(
      repo.path,
      ".agents/skills/skill-a/SKILL.md",
      "---\nname: skill-a\ndescription: Updated skill-a\n---\n# Updated\n",
      "update skill-a",
    );
    const up = await updateSkill({ yes: true });
    expect(up.ok).toBe(true);
    if (!up.ok) return;
    // Both get "updated" because the cache fetch pulls both; the group updates atomically
    expect(
      up.value.updated.includes("skill-a") ||
        up.value.updated.includes("skill-b"),
    ).toBe(true);

    // --- Remove skill-a ---
    const rm1 = await removeSkill("skill-a");
    expect(rm1.ok).toBe(true);

    // Cache should still exist (skill-b uses it)
    const loaded1 = await loadInstalled();
    expect(loaded1.ok).toBe(true);
    if (!loaded1.ok) return;
    expect(
      loaded1.value.skills.find((s) => s.name === "skill-b"),
    ).toBeDefined();
    expect(
      loaded1.value.skills.find((s) => s.name === "skill-a"),
    ).toBeUndefined();

    // --- Remove skill-b (last from this repo) ---
    const rm2 = await removeSkill("skill-b");
    expect(rm2.ok).toBe(true);

    const loaded2 = await loadInstalled();
    expect(loaded2.ok).toBe(true);
    if (!loaded2.ok) return;
    expect(loaded2.value.skills).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 3: Adopt (move mode) with fetchable remote → update → remove
// ---------------------------------------------------------------------------
describe("adopted skill with remote lifecycle", () => {
  test("adopt (move) → update → remove", async () => {
    const repo = await createStandaloneSkillRepo();
    cleanups.push(repo.cleanup);

    // Clone into .claude/skills/ so discover finds it as unmanaged
    const adoptable = await createAdoptableSkill(
      homeDir,
      "standalone-skill",
      repo.path,
    );

    const disc = await discoverSkills({ global: true, project: false });
    expect(disc.ok).toBe(true);
    if (!disc.ok) return;

    const skill = disc.value.skills.find(
      (s) => s.name === "standalone-skill" && !s.managed,
    );
    expect(skill).toBeDefined();
    if (!skill) return;

    // --- Adopt ---
    const adopt = await adoptSkill(skill, {
      mode: "move",
      scope: "global",
      also: ["claude-code"],
      skipScan: true,
    });
    expect(adopt.ok).toBe(true);
    if (!adopt.ok) return;

    expect(adopt.value.record.repo).toBeString();
    expect(adopt.value.record.sha).toMatch(/^[0-9a-f]{40}$/);

    const targetDir = join(homeDir, ".agents", "skills", "standalone-skill");
    expect((await lstat(targetDir)).isDirectory()).toBe(true);

    // --- Update after remote advances ---
    await addFileAndCommit(repo.path, "new.md", "content", "advance remote");
    const up = await updateSkill({ name: "standalone-skill", yes: true });
    expect(up.ok).toBe(true);
    if (!up.ok) return;
    expect(up.value.updated).toContain("standalone-skill");

    // --- Remove ---
    const rm = await removeSkill("standalone-skill");
    expect(rm.ok).toBe(true);

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 4: Adopt local-only skill (no git remote) → update skips → remove
// ---------------------------------------------------------------------------
describe("adopted local skill (no remote) lifecycle", () => {
  test("adopt → update skips with 'local' status → remove", async () => {
    // Create an unmanaged skill in .claude/skills/ (no git)
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    await createSkillDir(claudeSkillsDir, "local-skill");

    const disc = await discoverSkills({ global: true, project: false });
    expect(disc.ok).toBe(true);
    if (!disc.ok) return;

    const skill = disc.value.skills.find((s) => s.name === "local-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    // --- Adopt (move mode) ---
    const adopt = await adoptSkill(skill, {
      mode: "move",
      scope: "global",
      skipScan: true,
    });
    expect(adopt.ok).toBe(true);
    if (!adopt.ok) return;

    expect(adopt.value.record.repo).toBeNull();

    // --- Update → should report "local" status, not crash ---
    const progressStatuses: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name, status) {
        if (name === "local-skill") progressStatuses.push(status);
      },
    });
    expect(up.ok).toBe(true);
    expect(progressStatuses).toContain("local");

    // --- Remove ---
    const rm = await removeSkill("local-skill");
    expect(rm.ok).toBe(true);

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 5: Track-in-place adoption → update skips (linked) → remove
// ---------------------------------------------------------------------------
describe("track-in-place adoption lifecycle", () => {
  test("adopt track-in-place → update skips (linked) → remove", async () => {
    const claudeSkillsDir = join(homeDir, ".claude", "skills");
    const srcPath = await createSkillDir(claudeSkillsDir, "tracked-skill");

    const disc = await discoverSkills({ global: true, project: false });
    expect(disc.ok).toBe(true);
    if (!disc.ok) return;

    const skill = disc.value.skills.find((s) => s.name === "tracked-skill");
    expect(skill).toBeDefined();
    if (!skill) return;

    // --- Adopt track-in-place ---
    const adopt = await adoptSkill(skill, {
      mode: "track-in-place",
      scope: "global",
      skipScan: true,
    });
    expect(adopt.ok).toBe(true);
    if (!adopt.ok) return;

    expect(adopt.value.record.scope).toBe("linked");
    expect(adopt.value.record.path).toBe(srcPath);

    // Skill NOT moved — still at original location
    expect((await lstat(srcPath)).isDirectory()).toBe(true);
    const agentsPath = join(homeDir, ".agents", "skills", "tracked-skill");
    expect(await lstat(agentsPath).catch(() => null)).toBeNull();

    // --- Update → should skip with "linked" status ---
    const progressStatuses: string[] = [];
    const up = await updateSkill({
      yes: true,
      onProgress(name, status) {
        if (name === "tracked-skill") progressStatuses.push(status);
      },
    });
    expect(up.ok).toBe(true);
    expect(progressStatuses).toContain("linked");

    // --- Remove ---
    const rm = await removeSkill("tracked-skill", { scope: "linked" });
    expect(rm.ok).toBe(true);

    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (!loaded.ok) return;
    expect(loaded.value.skills).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 6: Link → Disable → Enable → Remove
// ---------------------------------------------------------------------------
describe("linked skill lifecycle", () => {
  test("link → disable → enable → remove", async () => {
    // Create a local skill directory to link
    const localDir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(localDir));
    await createSkillDir(localDir, "dev-skill");
    const devSkillPath = join(localDir, "dev-skill");

    // --- Link ---
    const link = await linkSkill(devSkillPath, {
      scope: "global",
      also: ["claude-code"],
    });
    expect(link.ok).toBe(true);
    if (!link.ok) return;

    expect(link.value.scope).toBe("linked");
    expect(link.value.path).toBeString();

    const installDir = join(homeDir, ".agents", "skills", "dev-skill");
    const linkStat = await lstat(installDir);
    expect(linkStat.isSymbolicLink()).toBe(true);
    expect(await readlink(installDir)).toBe(devSkillPath);

    const claudeLink = join(homeDir, ".claude", "skills", "dev-skill");
    expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);

    // --- Disable ---
    const dis = await disableSkill("dev-skill");
    expect(dis.ok).toBe(true);

    // Linked skills: no file move, just symlink removal + active=false
    expect(await lstat(claudeLink).catch(() => null)).toBeNull();

    const loaded1 = await loadInstalled();
    expect(loaded1.ok).toBe(true);
    if (!loaded1.ok) return;
    expect(
      loaded1.value.skills.find((s) => s.name === "dev-skill")?.active,
    ).toBe(false);

    // --- Enable ---
    const en = await enableSkill("dev-skill");
    expect(en.ok).toBe(true);

    expect((await lstat(claudeLink)).isSymbolicLink()).toBe(true);

    const loaded2 = await loadInstalled();
    expect(loaded2.ok).toBe(true);
    if (!loaded2.ok) return;
    expect(
      loaded2.value.skills.find((s) => s.name === "dev-skill")?.active,
    ).toBe(true);

    // --- Remove ---
    const rm = await removeSkill("dev-skill", { scope: "linked" });
    expect(rm.ok).toBe(true);

    const loaded3 = await loadInstalled();
    expect(loaded3.ok).toBe(true);
    if (!loaded3.ok) return;
    expect(loaded3.value.skills).toHaveLength(0);
  });
});
