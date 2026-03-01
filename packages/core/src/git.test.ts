import { describe, test, expect, afterEach } from "bun:test"
import { clone, pull, fetch, diff, revParse, log } from "./git"
import { makeTmpDir, removeTmpDir } from "./fs"
import { createStandaloneSkillRepo } from "@skilltap/test-utils"

describe("clone", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null
  let dest: string | null = null

  afterEach(async () => {
    if (dest) { await removeTmpDir(dest); dest = null }
    if (repo) { await repo.cleanup(); repo = null }
  })

  test("clones a local repo successfully", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    const result = await clone(repo.path, dest + "/clone")
    expect(result.ok).toBe(true)

    const skillMd = Bun.file(dest + "/clone/SKILL.md")
    expect(await skillMd.exists()).toBe(true)
  })

  test("returns GitError for invalid URL", async () => {
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    const result = await clone("https://invalid.invalid/no/such/repo.git", dest + "/clone")
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.message).toContain("git clone failed")
  })
})

describe("revParse", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null
  let dest: string | null = null

  afterEach(async () => {
    if (dest) { await removeTmpDir(dest); dest = null }
    if (repo) { await repo.cleanup(); repo = null }
  })

  test("returns a 40-char SHA after clone", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    await clone(repo.path, dest + "/clone")
    const result = await revParse(dest + "/clone")
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.value).toMatch(/^[0-9a-f]{40}$/)
  })
})

describe("log", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null
  let dest: string | null = null

  afterEach(async () => {
    if (dest) { await removeTmpDir(dest); dest = null }
    if (repo) { await repo.cleanup(); repo = null }
  })

  test("returns commit entries with sha, message, date", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    await clone(repo.path, dest + "/clone")
    const result = await log(dest + "/clone", 5)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.value.length).toBeGreaterThan(0)
    const entry = result.value[0]!
    expect(entry.sha).toMatch(/^[0-9a-f]{40}$/)
    expect(typeof entry.message).toBe("string")
    expect(typeof entry.date).toBe("string")
    expect(entry.date.length).toBeGreaterThan(0)
  })
})

describe("pull and fetch", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null
  let dest: string | null = null

  afterEach(async () => {
    if (dest) { await removeTmpDir(dest); dest = null }
    if (repo) { await repo.cleanup(); repo = null }
  })

  test("pull succeeds on an already-cloned repo", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    await clone(repo.path, dest + "/clone")
    const result = await pull(dest + "/clone")
    expect(result.ok).toBe(true)
  })

  test("fetch succeeds on an already-cloned repo", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    await clone(repo.path, dest + "/clone")
    const result = await fetch(dest + "/clone")
    expect(result.ok).toBe(true)
  })
})

describe("diff", () => {
  let repo: { path: string; cleanup: () => Promise<void> } | null = null
  let dest: string | null = null

  afterEach(async () => {
    if (dest) { await removeTmpDir(dest); dest = null }
    if (repo) { await repo.cleanup(); repo = null }
  })

  test("returns empty string when comparing HEAD to itself", async () => {
    repo = await createStandaloneSkillRepo()
    const destResult = await makeTmpDir()
    expect(destResult.ok).toBe(true)
    if (!destResult.ok) return
    dest = destResult.value

    await clone(repo.path, dest + "/clone")
    const sha = await revParse(dest + "/clone")
    expect(sha.ok).toBe(true)
    if (!sha.ok) return

    const result = await diff(dest + "/clone", sha.value, sha.value)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.value).toBe("")
  })
})
