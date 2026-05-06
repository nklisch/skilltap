import { rename, unlink } from "node:fs/promises";
import { stringify } from "smol-toml";
import { err, ok, type Result, UserError } from "../types";
import { manifestPath } from "./paths";
import type { ProjectManifest } from "./schemas";

// Atomic write: stringify → write tmp → rename over original.
// Prevents readers from seeing a partial file if the process is interrupted.
export async function saveManifest(
  projectRoot: string,
  manifest: ProjectManifest,
): Promise<Result<void, UserError>> {
  const target = manifestPath(projectRoot);
  const tmp = `${target}.tmp`;

  let text: string;
  try {
    text = stringify(manifest as unknown as Record<string, unknown>);
  } catch (e) {
    return err(new UserError(`Failed to serialize manifest: ${e}`));
  }

  try {
    await Bun.write(tmp, text);
  } catch (e) {
    return err(new UserError(`Failed to write ${tmp}: ${e}`));
  }

  try {
    await rename(tmp, target);
  } catch (e) {
    // best-effort cleanup of tmp on rename failure
    await unlink(tmp).catch(() => undefined);
    return err(new UserError(`Failed to move ${tmp} → ${target}: ${e}`));
  }

  return ok(undefined);
}
