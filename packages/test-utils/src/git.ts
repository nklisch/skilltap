import { $ } from "bun";

export async function initRepo(dir: string): Promise<void> {
  await $`git -C ${dir} init`.quiet();
  await $`git -C ${dir} config user.email "test@skilltap.test"`.quiet();
  await $`git -C ${dir} config user.name "Skilltap Test"`.quiet();
}

export async function commitAll(
  dir: string,
  message = "initial commit",
): Promise<string> {
  await $`git -C ${dir} add -A`.quiet();
  await $`git -C ${dir} commit -m ${message}`.quiet();
  const result = await $`git -C ${dir} rev-parse HEAD`.quiet();
  return result.stdout.toString().trim();
}

export async function addFileAndCommit(
  dir: string,
  filename: string,
  content: string,
  message = "add file",
): Promise<string> {
  await Bun.write(`${dir}/${filename}`, content);
  await $`git -C ${dir} add ${filename}`.quiet();
  await $`git -C ${dir} commit -m ${message}`.quiet();
  const result = await $`git -C ${dir} rev-parse HEAD`.quiet();
  return result.stdout.toString().trim();
}
