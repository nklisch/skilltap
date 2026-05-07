import { copyFile, writeFile } from "node:fs/promises";
import { lockfilePath, manifestPath } from "./paths";

/**
 * Recover a corrupt TOML file by backing it up to `<path>.bak` and writing
 * the given fresh content in its place. The backup is best-effort — if the
 * backup fails (e.g. file vanished mid-recovery), the rewrite still proceeds
 * so the caller is left with a valid file.
 *
 * Used by:
 * - `doctor --fix` when manifest/lockfile fail to load
 * - `skilltap install` interactive-mode preflight when a corrupt
 *   skilltap.toml is detected at install start (auto-recover before
 *   proceeding so the install doesn't leave the project in a half-managed
 *   state)
 */
async function recoverTomlFile(
  path: string,
  freshContent: string,
): Promise<void> {
  await copyFile(path, `${path}.bak`).catch(() => {});
  await writeFile(path, freshContent);
}

/** Recover a corrupt skilltap.toml: backup + reset to empty (all defaults). */
export async function recoverManifest(projectRoot: string): Promise<void> {
  await recoverTomlFile(manifestPath(projectRoot), "");
}

/** Recover a corrupt skilltap.lock: backup + reset to empty (version = 1). */
export async function recoverLockfile(projectRoot: string): Promise<void> {
  await recoverTomlFile(lockfilePath(projectRoot), "version = 1\n");
}
