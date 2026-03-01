import { describe, test, expect } from "bun:test"
import { makeTmpDir, removeTmpDir } from "./fs"

describe("makeTmpDir / removeTmpDir", () => {
  test("creates a directory that exists", async () => {
    const result = await makeTmpDir()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const dir = result.value
    expect(dir).toMatch(/^\/tmp\/skilltap-/)
    await Bun.write(dir + "/.keep", "")
    expect(await Bun.file(dir + "/.keep").exists()).toBe(true)
    await removeTmpDir(dir)
  })

  test("removeTmpDir removes the directory", async () => {
    const result = await makeTmpDir()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const dir = result.value
    await Bun.write(dir + "/file.txt", "hello")
    await removeTmpDir(dir)
    const { $ } = await import("bun")
    const check = await $`test -d ${dir}`.quiet().catch(() => null)
    expect(check).toBeNull()
  })

  test("removeTmpDir is a no-op for nonexistent path", async () => {
    await expect(removeTmpDir("/tmp/skilltap-does-not-exist-xyz")).resolves.toBeUndefined()
  })
})
