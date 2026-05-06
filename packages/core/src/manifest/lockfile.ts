import { rename, unlink } from "node:fs/promises";
import { parse, stringify } from "smol-toml";
import { parseWithResult } from "../schemas";
import { err, ok, type Result, UserError } from "../types";
import { lockfilePath } from "./paths";
import { type Lockfile, LockfileSchema } from "./schemas";

const DEFAULT_LOCKFILE: Lockfile = LockfileSchema.parse({ version: 1 });

export async function lockfileExists(projectRoot: string): Promise<boolean> {
  return await Bun.file(lockfilePath(projectRoot)).exists();
}

export async function loadLockfile(
  projectRoot: string,
): Promise<Result<Lockfile, UserError>> {
  const path = lockfilePath(projectRoot);
  const f = Bun.file(path);
  if (!(await f.exists())) return ok(DEFAULT_LOCKFILE);

  let text: string;
  try {
    text = await f.text();
  } catch (e) {
    return err(new UserError(`Failed to read ${path}: ${e}`));
  }

  let raw: unknown;
  try {
    raw = parse(text);
  } catch (e) {
    return err(new UserError(`Invalid TOML in ${path}: ${e}`));
  }

  return parseWithResult(LockfileSchema, raw, "skilltap.lock");
}

export async function saveLockfile(
  projectRoot: string,
  lockfile: Lockfile,
): Promise<Result<void, UserError>> {
  const target = lockfilePath(projectRoot);
  const tmp = `${target}.tmp`;

  let text: string;
  try {
    text = stringify(lockfile as unknown as Record<string, unknown>);
  } catch (e) {
    return err(new UserError(`Failed to serialize lockfile: ${e}`));
  }

  try {
    await Bun.write(tmp, text);
  } catch (e) {
    return err(new UserError(`Failed to write ${tmp}: ${e}`));
  }

  try {
    await rename(tmp, target);
  } catch (e) {
    await unlink(tmp).catch(() => undefined);
    return err(new UserError(`Failed to move ${tmp} → ${target}: ${e}`));
  }

  return ok(undefined);
}
