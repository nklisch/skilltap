import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { err, ok, type Result, UserError } from "./types";

// Leaf module: no internal imports beyond ./types. Lets state/save.ts and
// config.ts both depend on these helpers without creating a cycle.

export function getConfigDir(): string {
  const xdg = process.env.XDG_CONFIG_HOME;
  return xdg ? join(xdg, "skilltap") : join(homedir(), ".config", "skilltap");
}

export async function ensureDirs(): Promise<Result<void>> {
  const dir = getConfigDir();
  try {
    await mkdir(join(dir, "taps"), { recursive: true });
    await mkdir(join(dir, "cache"), { recursive: true });
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to create config directories: ${e}`));
  }
}
