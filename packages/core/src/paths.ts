import { lstat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { getConfigDir } from "./config";
import { globalBase } from "./fs";
import type { InstalledSkill } from "./schemas/installed";
import { AGENT_DEF_PATHS } from "./symlink";

export function scopeBase(
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
}

export async function findProjectRoot(startDir?: string): Promise<string> {
  let dir = startDir ?? process.cwd();
  while (true) {
    const stat = await lstat(join(dir, ".git")).catch(() => null);
    if (stat) return dir;
    const parent = dirname(dir);
    if (parent === dir) return startDir ?? process.cwd();
    dir = parent;
  }
}

// Like findProjectRoot, but returns null when no .git ancestor exists.
// Used by smart-scope-default logic to distinguish "in a git repo" from
// "outside any repo" (where findProjectRoot's cwd-fallback would mislead).
export async function isInGitRepo(startDir?: string): Promise<string | null> {
  let dir = startDir ?? process.cwd();
  while (true) {
    const stat = await lstat(join(dir, ".git")).catch(() => null);
    if (stat) return dir;
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

// Walk up from startDir looking for a directory containing skilltap.toml.
// Returns the directory if found, or null if no manifest ancestor exists.
// Used by commands like `sync` that operate on a manifest-rooted project,
// distinct from smart-scope-default's git-rooted check (isInGitRepo).
export async function findManifestRoot(
  startDir?: string,
): Promise<string | null> {
  let dir = startDir ?? process.cwd();
  while (true) {
    const stat = await lstat(join(dir, "skilltap.toml")).catch(() => null);
    if (stat) return dir;
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

export function skillInstallDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return join(scopeBase(scope, projectRoot), ".agents", "skills", name);
}

export function skillDisabledDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return join(
    scopeBase(scope, projectRoot),
    ".agents",
    "skills",
    ".disabled",
    name,
  );
}

export function agentDefPath(
  name: string,
  platform: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, `${name}.md`);
}

export function agentDefDisabledPath(
  name: string,
  platform: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, ".disabled", `${name}.md`);
}

export function currentSkillDir(
  record: Pick<InstalledSkill, "name" | "scope" | "active">,
  projectRoot?: string,
): string {
  const scope = record.scope as "global" | "project";
  return record.active === false
    ? skillDisabledDir(record.name, scope, projectRoot)
    : skillInstallDir(record.name, scope, projectRoot);
}

export function skillCacheDir(repoUrl: string): string {
  const hash = Bun.hash(repoUrl).toString(16);
  return join(getConfigDir(), "cache", hash);
}
