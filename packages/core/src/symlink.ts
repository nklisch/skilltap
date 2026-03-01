import { join } from "node:path"
import { mkdir, symlink, unlink } from "node:fs/promises"
import { ok, err, UserError } from "./types"
import type { Result } from "./types"
import { globalBase } from "./fs"

const AGENT_PATHS: Record<string, string> = {
  "claude-code": ".claude/skills",
  cursor: ".cursor/skills",
  codex: ".codex/skills",
  gemini: ".gemini/skills",
  windsurf: ".windsurf/skills",
}

export const VALID_AGENT_IDS: string[] = Object.keys(AGENT_PATHS)

function symlinkPath(
  skillName: string,
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_PATHS[agent]
  if (!relDir) return null
  const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd())
  return join(base, relDir, skillName)
}

export async function createAgentSymlinks(
  skillName: string,
  targetPath: string,
  agents: string[],
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  for (const agent of agents) {
    const linkPath = symlinkPath(skillName, agent, scope, projectRoot)
    if (!linkPath) {
      return err(new UserError(`Unknown agent identifier: "${agent}"`, `Valid agents: ${VALID_AGENT_IDS.join(", ")}`))
    }
    try {
      await mkdir(join(linkPath, ".."), { recursive: true })
      await symlink(targetPath, linkPath, "dir")
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      return err(new UserError(`Failed to create symlink for ${agent}: ${msg}`))
    }
  }
  return ok(undefined)
}

export async function removeAgentSymlinks(
  skillName: string,
  agents: string[],
  scope: "global" | "project" | "linked",
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  const effectiveScope = scope === "linked" ? "global" : scope
  for (const agent of agents) {
    const linkPath = symlinkPath(skillName, agent, effectiveScope, projectRoot)
    if (!linkPath) continue
    await unlink(linkPath).catch(() => {})
  }
  return ok(undefined)
}
