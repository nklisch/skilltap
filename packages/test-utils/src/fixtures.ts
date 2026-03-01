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
