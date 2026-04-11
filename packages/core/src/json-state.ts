import { z } from "zod/v4";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { parseWithResult } from "./schemas";
import { err, ok, type Result, UserError } from "./types";

export async function loadJsonState<T>(
  path: string,
  schema: z.ZodType<T>,
  label: string,
  defaultValue: T,
): Promise<Result<T, UserError>> {
  const f = Bun.file(path);
  if (!(await f.exists())) return ok(defaultValue);
  let raw: unknown;
  try {
    raw = await f.json();
  } catch (e) {
    return err(new UserError(`Invalid JSON in ${label}: ${e}`));
  }
  return parseWithResult(schema, raw, label);
}

export async function saveJsonState(
  path: string,
  data: unknown,
  label: string,
  projectRoot: string | undefined,
  ensureGlobalDirs: () => Promise<Result<void, UserError>>,
): Promise<Result<void, UserError>> {
  if (projectRoot) {
    try {
      await mkdir(join(projectRoot, ".agents"), { recursive: true });
    } catch (e) {
      return err(new UserError(`Failed to create .agents directory: ${e}`));
    }
  } else {
    const dirsResult = await ensureGlobalDirs();
    if (!dirsResult.ok) return dirsResult;
  }
  try {
    await Bun.write(path, JSON.stringify(data, null, 2));
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to save ${label}: ${e}`));
  }
}
