import { err, ok, UserError } from "../types";
import type { SourceAdapter } from "./types";

const LOCAL_PREFIXES = ["./", "/", "~/"];
const URL_PROTOCOLS = ["https://", "http://", "git@", "ssh://"];

export const githubAdapter: SourceAdapter = {
  name: "github",

  canHandle(source: string): boolean {
    if (source.startsWith("github:")) return true;
    if (LOCAL_PREFIXES.some((p) => source.startsWith(p))) return false;
    if (URL_PROTOCOLS.some((p) => source.startsWith(p))) return false;
    return source.includes("/");
  },

  async resolve(source: string) {
    let s = source.startsWith("github:")
      ? source.slice("github:".length)
      : source;

    // Extract @ref suffix
    let ref: string | undefined;
    const atIdx = s.lastIndexOf("@");
    if (atIdx !== -1) {
      ref = s.slice(atIdx + 1);
      s = s.slice(0, atIdx);
    }

    const parts = s.split("/").filter(Boolean);
    if (parts.length !== 2) {
      return err(
        new UserError(
          `Invalid GitHub source: "${source}"`,
          "Use format: owner/repo or github:owner/repo",
        ),
      );
    }

    const [owner, repo] = parts;
    const url = `https://github.com/${owner}/${repo}.git`;
    return ok({ url, ...(ref ? { ref } : {}), adapter: "github" });
  },
};
