import { stat } from "node:fs/promises";
import { join } from "node:path";
import { parse } from "smol-toml";
import {
  type SkilltapPluginManifest,
  SkilltapPluginManifestSchema,
} from "../skilltap-plugin/schema";
import { publishDir } from "./paths";

export interface PublishDiscovery {
  publishable: SkilltapPluginManifest[];
  rejected: { path: string; reason: string }[];
}

// Walks `<repoRoot>/.skilltap/` and reads every `*.toml` file.
// A manifest with `publish = true` and a valid schema goes into `publishable`.
// Invalid TOML, schema mismatches, or `publish = false` go into `rejected`.
// `.skilltap/` not present → both arrays empty.
export async function discoverPublishablePlugins(
  repoRoot: string,
): Promise<PublishDiscovery> {
  const dir = publishDir(repoRoot);
  const publishable: SkilltapPluginManifest[] = [];
  const rejected: { path: string; reason: string }[] = [];

  // Bun.Glob.scan errors on non-existent cwd. Bail early if the dir is absent.
  const dirStat = await stat(dir).catch(() => null);
  if (!dirStat || !dirStat.isDirectory()) {
    return { publishable, rejected };
  }

  const glob = new Bun.Glob("*.toml");
  for await (const relPath of glob.scan({ cwd: dir, dot: false })) {
    const path = join(dir, relPath);
    let text: string;
    try {
      text = await Bun.file(path).text();
    } catch (e) {
      rejected.push({ path, reason: `Failed to read: ${e}` });
      continue;
    }
    let raw: unknown;
    try {
      raw = parse(text);
    } catch (e) {
      rejected.push({ path, reason: `Invalid TOML: ${e}` });
      continue;
    }
    const parsed = SkilltapPluginManifestSchema.safeParse(raw);
    if (!parsed.success) {
      rejected.push({
        path,
        reason: `Schema mismatch: ${parsed.error.message}`,
      });
      continue;
    }
    if (!parsed.data.publish) {
      rejected.push({
        path,
        reason: `publish = false (or omitted) — not exposed to outside installers.`,
      });
      continue;
    }
    publishable.push(parsed.data);
  }

  return { publishable, rejected };
}
