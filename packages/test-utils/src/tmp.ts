import { $ } from "bun";

export async function makeTmpDir(): Promise<string> {
  const dir = `/tmp/skilltap-${crypto.randomUUID()}`;
  await $`mkdir -p ${dir}`.quiet();
  return dir;
}

export async function removeTmpDir(dir: string): Promise<void> {
  try {
    await $`rm -rf ${dir}`.quiet();
  } catch {
    // ignore — already gone
  }
}
