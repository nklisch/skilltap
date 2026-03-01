import { ok } from "../types";
import type { SourceAdapter } from "./types";

const URL_PREFIXES = ["https://", "http://", "git@", "ssh://"];

export const gitAdapter: SourceAdapter = {
  name: "git",

  canHandle(source: string): boolean {
    return URL_PREFIXES.some((p) => source.startsWith(p));
  },

  async resolve(source: string) {
    return ok({ url: source, adapter: "git" });
  },
};
