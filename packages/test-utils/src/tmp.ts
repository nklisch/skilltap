import { realpath } from "node:fs/promises";
import { $ } from "bun";

export async function makeTmpDir(): Promise<string> {
  const dir = `/tmp/skilltap-${crypto.randomUUID()}`;
  await $`mkdir -p ${dir}`.quiet();
  // realpath resolves macOS's /tmp -> /private/tmp symlink so callers
  // get the canonical path. Without this, scan/parse functions that
  // walk the directory return /private/tmp paths while tests that
  // remember the original /tmp path get equality mismatches.
  return await realpath(dir);
}

export async function removeTmpDir(dir: string): Promise<void> {
  try {
    await $`rm -rf ${dir}`.quiet();
  } catch {
    // ignore — already gone
  }
}
