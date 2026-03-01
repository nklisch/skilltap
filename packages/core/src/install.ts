import { join, dirname, relative } from "node:path"
import { homedir } from "node:os"
import { lstat, mkdir } from "node:fs/promises"
import { $ } from "bun"
import { ok, err, UserError, GitError } from "./types"
import type { Result } from "./types"
import { makeTmpDir, removeTmpDir } from "./fs"
import { clone, revParse } from "./git"
import { scan } from "./scanner"
import { resolveSource } from "./adapters"
import { loadInstalled, saveInstalled, getConfigDir } from "./config"
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink"
import type { InstalledSkill } from "./schemas/installed"
import type { ScannedSkill } from "./scanner"

export type InstallOptions = {
  scope: "global" | "project"
  projectRoot?: string
  skillNames?: string[]
  also?: string[]
  ref?: string
  tap?: string | null
}

export type RemoveOptions = {
  scope?: "global" | "project" | "linked"
  projectRoot?: string
}

function globalBase(): string {
  return process.env.SKILLTAP_HOME ?? homedir()
}

export async function findProjectRoot(startDir?: string): Promise<string> {
  let dir = startDir ?? process.cwd()
  while (true) {
    const stat = await lstat(join(dir, ".git")).catch(() => null)
    if (stat) return dir
    const parent = dirname(dir)
    if (parent === dir) return startDir ?? process.cwd()
    dir = parent
  }
}

function skillInstallDir(name: string, scope: "global" | "project", projectRoot?: string): string {
  const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd())
  return join(base, ".agents", "skills", name)
}

function skillCacheDir(repoUrl: string): string {
  const hash = Bun.hash(repoUrl).toString(16)
  return join(getConfigDir(), "cache", hash)
}

export async function installSkill(
  source: string,
  options: InstallOptions,
): Promise<Result<InstalledSkill[], UserError | GitError>> {
  const also = options.also ?? []
  const ref = options.ref

  // 1. Check already-installed
  const installedResult = await loadInstalled()
  if (!installedResult.ok) return installedResult
  const installed = installedResult.value

  // 2. Resolve source
  const resolvedResult = await resolveSource(source)
  if (!resolvedResult.ok) return resolvedResult
  const resolved = resolvedResult.value

  // 3. Create temp dir and clone
  const tmpResult = await makeTmpDir()
  if (!tmpResult.ok) return tmpResult
  const tmpDir = tmpResult.value

  try {
    const cloneResult = await clone(resolved.url, tmpDir, { branch: ref, depth: 1 })
    if (!cloneResult.ok) return cloneResult

    // 4. Get SHA
    const shaResult = await revParse(tmpDir)
    if (!shaResult.ok) return shaResult
    const sha = shaResult.value

    // 5. Scan for skills
    const scanned = await scan(tmpDir)
    if (scanned.length === 0) {
      return err(new UserError(`No SKILL.md found in "${source}". This repo doesn't contain any skills.`))
    }

    // 6. Select skills to install
    const selected: ScannedSkill[] = options.skillNames
      ? options.skillNames.map((name) => {
          const found = scanned.find((s) => s.name === name)
          if (!found) throw new UserError(`Skill "${name}" not found in repo. Available: ${scanned.map((s) => s.name).join(", ")}`)
          return found
        })
      : scanned

    // 7. Check for already-installed conflicts
    for (const skill of selected) {
      const conflict = installed.skills.find((s) => s.name === skill.name && s.scope === options.scope)
      if (conflict) {
        return err(new UserError(
          `Skill '${skill.name}' is already installed.`,
          `Use 'skilltap update ${skill.name}' to update, or 'skilltap remove ${skill.name}' first.`,
        ))
      }
    }

    // 8. Determine standalone vs multi-skill
    // Standalone: single skill at repo root (skill.path === tmpDir)
    const isStandalone = scanned.length === 1 && scanned[0]!.path === tmpDir

    // 9. Place skills
    const now = new Date().toISOString()
    const newRecords: InstalledSkill[] = []

    if (isStandalone) {
      const skill = selected[0]!
      const destDir = skillInstallDir(skill.name, options.scope, options.projectRoot)
      await mkdir(dirname(destDir), { recursive: true })
      await $`mv ${tmpDir} ${destDir}`.quiet()

      await createAgentSymlinks(skill.name, destDir, also, options.scope, options.projectRoot)

      newRecords.push({
        name: skill.name,
        repo: resolved.url,
        ref: ref ?? null,
        sha,
        scope: options.scope,
        path: null,
        tap: options.tap ?? null,
        also,
        installedAt: now,
        updatedAt: now,
      })
    } else {
      // Multi-skill: move clone to cache, copy selected skills to install dirs
      const cacheRoot = skillCacheDir(resolved.url)
      await mkdir(dirname(cacheRoot), { recursive: true })
      await $`mv ${tmpDir} ${cacheRoot}`.quiet()

      for (const skill of selected) {
        const relPath = relative(cacheRoot, skill.path.replace(tmpDir, cacheRoot))
        const skillSrcInCache = skill.path.replace(tmpDir, cacheRoot)
        const destDir = skillInstallDir(skill.name, options.scope, options.projectRoot)
        await mkdir(dirname(destDir), { recursive: true })
        await $`cp -r ${skillSrcInCache} ${destDir}`.quiet()

        await createAgentSymlinks(skill.name, destDir, also, options.scope, options.projectRoot)

        newRecords.push({
          name: skill.name,
          repo: resolved.url,
          ref: ref ?? null,
          sha,
          scope: options.scope,
          path: relPath,
          tap: options.tap ?? null,
          also,
          installedAt: now,
          updatedAt: now,
        })
      }
    }

    // 10. Save installed.json
    installed.skills.push(...newRecords)
    const saveResult = await saveInstalled(installed)
    if (!saveResult.ok) return saveResult

    return ok(newRecords)
  } catch (e) {
    if (e instanceof UserError) return err(e)
    if (e instanceof GitError) return err(e)
    return err(new UserError(`Install failed: ${e instanceof Error ? e.message : String(e)}`))
  } finally {
    await removeTmpDir(tmpDir)
  }
}

export async function removeSkill(
  name: string,
  options: RemoveOptions = {},
): Promise<Result<void, UserError>> {
  const installedResult = await loadInstalled()
  if (!installedResult.ok) return installedResult
  const installed = installedResult.value

  const idx = installed.skills.findIndex((s) => {
    if (s.name !== name) return false
    if (options.scope && s.scope !== options.scope) return false
    return true
  })

  if (idx === -1) {
    return err(new UserError(`Skill '${name}' is not installed.`, `Run 'skilltap list' to see installed skills.`))
  }

  const record = installed.skills[idx]!

  // Remove agent symlinks
  await removeAgentSymlinks(record.name, record.also, record.scope, options.projectRoot)

  // Remove skill directory
  const installPath = skillInstallDir(record.name, record.scope === "linked" ? "global" : record.scope, options.projectRoot)
  await $`rm -rf ${installPath}`.quiet()

  // Remove cache if this was the last skill from the repo
  if (record.path !== null && record.repo) {
    const remainingFromSameRepo = installed.skills.filter(
      (s, i) => i !== idx && s.repo === record.repo,
    )
    if (remainingFromSameRepo.length === 0) {
      const cacheRoot = skillCacheDir(record.repo)
      await $`rm -rf ${cacheRoot}`.quiet()
    }
  }

  installed.skills.splice(idx, 1)
  const saveResult = await saveInstalled(installed)
  if (!saveResult.ok) return saveResult

  return ok(undefined)
}
