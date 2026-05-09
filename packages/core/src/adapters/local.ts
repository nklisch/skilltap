import { stat } from "node:fs/promises";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { err, ok, UserError } from "../types";
import type { SourceAdapter } from "./types";

// Strip a trailing :<plugin-name> or :* selector from a local path. The rule:
// the LAST `:`, but only if what follows it does NOT contain `/`. Local paths
// may contain `:` in their middle (rare but valid on Linux), so the guard is
// important.
function stripPluginSelector(input: string): {
  path: string;
  pluginSelector?: string;
} {
  const colonIdx = input.lastIndexOf(":");
  if (colonIdx <= 0) return { path: input };
  const tail = input.slice(colonIdx + 1);
  if (tail.length === 0 || tail.includes("/")) return { path: input };
  return { path: input.slice(0, colonIdx), pluginSelector: tail };
}

export const localAdapter: SourceAdapter = {
  name: "local",

  canHandle(source: string): boolean {
    return (
      source.startsWith("./") ||
      source.startsWith("/") ||
      source.startsWith("~/")
    );
  },

  async resolve(source: string) {
    const { path: rawPath, pluginSelector } = stripPluginSelector(source);
    const expanded = rawPath.startsWith("~/")
      ? resolve(homedir(), rawPath.slice(2))
      : resolve(rawPath);

    let stats: Awaited<ReturnType<typeof stat>>;
    try {
      stats = await stat(expanded);
    } catch {
      return err(new UserError(`Path does not exist: ${expanded}`));
    }

    if (!stats.isDirectory()) {
      return err(new UserError(`Path is not a directory: ${expanded}`));
    }

    return ok({
      url: expanded,
      ...(pluginSelector ? { pluginSelector } : {}),
      adapter: "local",
    });
  },
};
