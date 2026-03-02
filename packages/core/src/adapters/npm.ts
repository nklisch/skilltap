import {
  fetchPackageMetadata,
  parseNpmSource,
  resolveVersion,
} from "../npm-registry";
import { err, UserError } from "../types";
import type { SourceAdapter } from "./types";

export const npmAdapter: SourceAdapter = {
  name: "npm",

  canHandle(source: string): boolean {
    return source.startsWith("npm:");
  },

  async resolve(source: string) {
    const { name, version } = parseNpmSource(source);

    const metaResult = await fetchPackageMetadata(name);
    if (!metaResult.ok) {
      return err(new UserError(metaResult.error.message, metaResult.error.hint));
    }

    const versionResult = resolveVersion(metaResult.value, version);
    if (!versionResult.ok) return versionResult;

    const info = versionResult.value;
    return {
      ok: true,
      value: {
        url: info.dist.tarball,
        ref: info.version,
        adapter: "npm",
        integrity: info.dist.integrity || undefined,
      },
    };
  },
};
