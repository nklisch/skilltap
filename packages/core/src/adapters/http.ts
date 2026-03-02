import { err, ok, UserError } from "../types";
import type { SourceAdapter } from "./types";

/**
 * Handles direct tarball URL sources from HTTP registries.
 * These are identified by the "url:" prefix, added by loadTaps() when mapping
 * registry skills with source.type === "url" to TapSkill.repo strings.
 */
export const httpAdapter: SourceAdapter = {
  name: "http",

  canHandle(source: string): boolean {
    return source.startsWith("url:");
  },

  async resolve(source: string) {
    const url = source.slice(4); // strip "url:" prefix
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
      return err(new UserError(`Invalid URL source: "${source}"`));
    }
    return ok({ url, adapter: "http" });
  },
};
