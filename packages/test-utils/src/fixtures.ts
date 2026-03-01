import { join, dirname } from "path"
import { makeTmpDir, removeTmpDir } from "./tmp"
import { initRepo, commitAll } from "./git"

const FIXTURES_DIR = join(dirname(import.meta.dir), "fixtures")

type FixtureRepo = {
  path: string
  cleanup: () => Promise<void>
}

async function copyFixtureDir(fixtureName: string, destDir: string): Promise<void> {
  const srcDir = join(FIXTURES_DIR, fixtureName)
  const glob = new Bun.Glob("**/*")
  for await (const relPath of glob.scan({ cwd: srcDir, onlyFiles: true, dot: true })) {
    const src = join(srcDir, relPath)
    const dest = join(destDir, relPath)
    await Bun.write(dest, Bun.file(src))
  }
}

export async function createStandaloneSkillRepo(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("standalone-skill", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}

export async function createMultiSkillRepo(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("multi-skill-repo", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}

export async function createSampleTap(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("sample-tap", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}

export async function createMaliciousSkillRepo(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("malicious-skill", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}
