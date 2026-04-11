import { lstat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { getConfigDir } from "./config";
import { globalBase } from "./fs";
import { AGENT_DEF_PATHS } from "./symlink";

export function scopeBase(scope: "global" | "project", projectRoot?: string): string {
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
  return join(scopeBase(scope, projectRoot), ".agents", "skills", ".disabled", name);
}

export function agentDefPath(
  name: string,
  platform: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, name + ".md");
}

export function agentDefDisabledPath(
  name: string,
  platform: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, ".disabled", name + ".md");
}

export function skillCacheDir(repoUrl: string): string {
  const hash = Bun.hash(repoUrl).toString(16);
  return join(getConfigDir(), "cache", hash);
}
