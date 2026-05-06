import { join } from "node:path";

export const MANIFEST_FILENAME = "skilltap.toml";
export const LOCKFILE_FILENAME = "skilltap.lock";
export const PUBLISH_DIR = ".skilltap";

export function manifestPath(projectRoot: string): string {
  return join(projectRoot, MANIFEST_FILENAME);
}

export function lockfilePath(projectRoot: string): string {
  return join(projectRoot, LOCKFILE_FILENAME);
}

export function publishDir(projectRoot: string): string {
  return join(projectRoot, PUBLISH_DIR);
}
