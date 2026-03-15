import { lstat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { getConfigDir } from "./config";
import { globalBase } from "./fs";

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
  const base =
    scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  return join(base, ".agents", "skills", name);
}

export function skillDisabledDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  const base =
    scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  return join(base, ".agents", "skills", ".disabled", name);
}

export function skillCacheDir(repoUrl: string): string {
  const hash = Bun.hash(repoUrl).toString(16);
  return join(getConfigDir(), "cache", hash);
}
