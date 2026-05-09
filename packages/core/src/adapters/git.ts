import { ok } from "../types";
import type { SourceAdapter } from "./types";

const URL_PREFIXES = ["https://", "http://", "git@", "ssh://"];

// Strip a trailing :<plugin-name> or :* from a git URL while preserving
// the URL's intrinsic colons (https://, git@host:owner/repo, ssh://...).
// The rule: take the LAST `:`; if what follows it does NOT contain `/`,
// treat it as a plugin selector.
function stripPluginSelector(url: string): {
  url: string;
  pluginSelector?: string;
} {
  const colonIdx = url.lastIndexOf(":");
  if (colonIdx <= 0) return { url };
  const tail = url.slice(colonIdx + 1);
  if (tail.length === 0 || tail.includes("/")) return { url };
  return { url: url.slice(0, colonIdx), pluginSelector: tail };
}

export const gitAdapter: SourceAdapter = {
  name: "git",

  canHandle(source: string): boolean {
    return URL_PREFIXES.some((p) => source.startsWith(p));
  },

  async resolve(source: string) {
    // Strip the trailing `:selector` first so a subsequent `@ref` parse
    // doesn't accidentally swallow the selector.
    const stripped = stripPluginSelector(source);
    let s = stripped.url;
    let ref: string | undefined;
    const atIdx = s.lastIndexOf("@");
    // git@host:owner/repo URLs start with `git@` — that `@` is at index 3
    // and is part of the URL prefix, not a ref. Only strip refs whose `@`
    // appears AFTER the host portion (i.e. not inside the prefix).
    if (atIdx > 4 && !s.slice(atIdx).includes("/")) {
      ref = s.slice(atIdx + 1);
      s = s.slice(0, atIdx);
    }
    return ok({
      url: s,
      ...(ref ? { ref } : {}),
      ...(stripped.pluginSelector
        ? { pluginSelector: stripped.pluginSelector }
        : {}),
      adapter: "git",
    });
  },
};
