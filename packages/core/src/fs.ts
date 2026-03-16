import { homedir } from "node:os";
import { lstat, stat } from "node:fs/promises";
import { $ } from "bun";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export function globalBase(): string {
  return process.env.SKILLTAP_HOME ?? homedir();
}

export async function makeTmpDir(): Promise<Result<string, UserError>> {
  const dir = `/tmp/skilltap-${crypto.randomUUID()}`;
  try {
    await $`mkdir -p ${dir}`.quiet();
    return ok(dir);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return err(new UserError(`failed to create temp dir: ${msg}`));
  }
}

export async function removeTmpDir(dir: string): Promise<void> {
  try {
    await $`rm -rf ${dir}`.quiet();
  } catch {
    // ignore
  }
}

export async function resolvedDirExists(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isDirectory();
  } catch {
    return false;
  }
}

export async function fileExists(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isFile();
  } catch {
    return false;
  }
}

export async function isSymlinkAt(path: string): Promise<boolean> {
  try {
    return (await lstat(path)).isSymbolicLink();
  } catch {
    return false;
  }
}
