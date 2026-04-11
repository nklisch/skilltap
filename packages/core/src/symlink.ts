import { lstat, mkdir, readlink, rm, symlink, unlink } from "node:fs/promises";
import { join } from "node:path";
import { globalBase } from "./fs";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

/**
 * Remove an existing path so a symlink can be created there.
 * - If it's a symlink pointing to the same target, returns false (no action needed).
 * - If it's a symlink pointing elsewhere, unlinks it and returns true.
 * - If it's a real directory/file, removes it and returns true.
 * - If nothing exists, returns true (safe to create).
 */
async function clearForSymlink(
  linkPath: string,
  targetPath: string,
): Promise<boolean> {
  let stat: Awaited<ReturnType<typeof lstat>>;
  try {
    stat = await lstat(linkPath);
  } catch {
    return true; // Nothing exists — safe to create
  }

  if (stat.isSymbolicLink()) {
    const existing = await readlink(linkPath);
    if (existing === targetPath) return false; // Already correct
    await unlink(linkPath);
    return true;
  }

  // Real file or directory — remove it to make way for the symlink
  await rm(linkPath, { recursive: true, force: true });
  return true;
}

export const AGENT_PATHS: Record<string, string> = {
  "claude-code": ".claude/skills",
  cursor: ".cursor/skills",
  codex: ".codex/skills",
  gemini: ".gemini/skills",
  windsurf: ".windsurf/skills",
};

export const AGENT_DEF_PATHS: Record<string, string> = {
  "claude-code": ".claude/agents",
};

export const AGENT_LABELS: Record<string, string> = {
  "claude-code": "Claude Code",
  cursor: "Cursor",
  codex: "Codex",
  gemini: "Gemini",
  windsurf: "Windsurf",
};

export const VALID_AGENT_IDS: string[] = Object.keys(AGENT_PATHS);

function symlinkPath(
  skillName: string,
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_PATHS[agent];
  if (!relDir) return null;
  const base =
    scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  return join(base, relDir, skillName);
}

export async function createAgentSymlinks(
  skillName: string,
  targetPath: string,
  agents: string[],
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  for (const agent of agents) {
    const linkPath = symlinkPath(skillName, agent, scope, projectRoot);
    if (!linkPath) {
      return err(
        new UserError(
          `Unknown agent identifier: "${agent}"`,
          `Valid agents: ${VALID_AGENT_IDS.join(", ")}`,
        ),
      );
    }
    try {
      await mkdir(join(linkPath, ".."), { recursive: true });
      const needed = await clearForSymlink(linkPath, targetPath);
      if (needed) {
        await symlink(targetPath, linkPath, "dir");
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      return err(
        new UserError(`Failed to create symlink for ${agent}: ${msg}`),
      );
    }
  }
  return ok(undefined);
}

export async function removeAgentSymlinks(
  skillName: string,
  agents: string[],
  scope: "global" | "project" | "linked",
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  const effectiveScope = scope === "linked" ? (projectRoot ? "project" : "global") : scope;
  for (const agent of agents) {
    const linkPath = symlinkPath(skillName, agent, effectiveScope, projectRoot);
    if (!linkPath) continue;
    await unlink(linkPath).catch(() => {});
  }
  return ok(undefined);
}
