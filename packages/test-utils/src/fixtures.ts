import { dirname, join } from "node:path";
import { commitAll, initRepo } from "./git";
import { makeTmpDir, removeTmpDir } from "./tmp";

const FIXTURES_DIR = join(dirname(import.meta.dir), "fixtures");

type FixtureRepo = {
  path: string;
  cleanup: () => Promise<void>;
};

async function copyFixtureDir(
  fixtureName: string,
  destDir: string,
): Promise<void> {
  const srcDir = join(FIXTURES_DIR, fixtureName);
  const glob = new Bun.Glob("**/*");
  for await (const relPath of glob.scan({
    cwd: srcDir,
    onlyFiles: true,
    dot: true,
  })) {
    const src = join(srcDir, relPath);
    const dest = join(destDir, relPath);
    await Bun.write(dest, Bun.file(src));
  }
}

async function createFixtureRepo(fixtureName: string): Promise<FixtureRepo> {
  const path = await makeTmpDir();
  await copyFixtureDir(fixtureName, path);
  await initRepo(path);
  await commitAll(path);
  return { path, cleanup: () => removeTmpDir(path) };
}

export const createStandaloneSkillRepo = () =>
  createFixtureRepo("standalone-skill");
export const createMultiSkillRepo = () => createFixtureRepo("multi-skill-repo");
export const createSampleTap = () => createFixtureRepo("sample-tap");
export const createMaliciousSkillRepo = () =>
  createFixtureRepo("malicious-skill");
export const createClaudePluginRepo = () => createFixtureRepo("claude-plugin");
export const createCodexPluginRepo = () => createFixtureRepo("codex-plugin");
export const createTapWithPlugins = () => createFixtureRepo("tap-with-plugins");

/**
 * Creates a bare skill directory (not a git repo) with a SKILL.md file.
 * Useful for adopt and link tests where no git history is needed.
 */
export async function createSkillDir(
  baseDir: string,
  name: string,
  content?: string,
): Promise<string> {
  const { mkdir } = await import("node:fs/promises");
  const skillDir = join(baseDir, name);
  await mkdir(skillDir, { recursive: true });
  const md =
    content ??
    `---\nname: ${name}\ndescription: A test skill\n---\n# ${name}\nTest content.\n`;
  await Bun.write(join(skillDir, "SKILL.md"), md);
  return skillDir;
}

/**
 * Creates an adoptable skill: a git clone inside `homeDir/.claude/skills/<name>/`
 * whose origin points to `remoteRepoPath` (a fixture repo). This enables adopt
 * tests where the adopted skill has a fetchable remote for subsequent updates.
 */
export async function createAdoptableSkill(
  homeDir: string,
  skillName: string,
  remoteRepoPath: string,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const { mkdir } = await import("node:fs/promises");
  const { $ } = await import("bun");
  const claudeSkillsDir = join(homeDir, ".claude", "skills");
  await mkdir(claudeSkillsDir, { recursive: true });
  const skillDir = join(claudeSkillsDir, skillName);
  await $`git clone ${remoteRepoPath} ${skillDir}`.quiet();
  return { path: skillDir, cleanup: () => removeTmpDir(skillDir) };
}
