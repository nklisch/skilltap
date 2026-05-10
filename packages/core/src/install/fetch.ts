import { realpath } from "node:fs/promises";
import { $ } from "bun";
import { clone, revParse } from "../git";
import { downloadAndExtract } from "../npm-registry";
import type { ResolvedSource } from "../schemas/agent";
import { wrapShell } from "../shell";
import type { Result, GitError, NetworkError, UserError } from "../types";
import { ok } from "../types";

export type FetchedContent = {
  contentDir: string;
  sha: string | null;
  cloneUrl: string | undefined;
};

/** Fetch skill content into tmpDir based on adapter type. Returns contentDir and optional sha. */
export async function fetchContent(
  resolved: ResolvedSource,
  tmpDir: string,
  effectiveRef: string | undefined,
): Promise<Result<FetchedContent, UserError | GitError | NetworkError>> {
  let contentDir: string;
  let sha: string | null;
  let cloneUrl: string | undefined;

  if (resolved.adapter === "npm") {
    const extractResult = await downloadAndExtract(
      resolved.url,
      tmpDir,
      resolved.integrity,
    );
    if (!extractResult.ok) return extractResult;
    contentDir = extractResult.value;
    sha = null;
  } else if (resolved.adapter === "local") {
    // Local paths: try git clone first (preserves update capability), fall back to cp
    const isGitRepo = await $`git -C ${resolved.url} rev-parse --git-dir`
      .quiet()
      .then(() => true)
      .catch(() => false);
    if (isGitRepo) {
      const cloneResult = await clone(resolved.url, tmpDir, {
        branch: effectiveRef,
        depth: 1,
      });
      if (!cloneResult.ok) return cloneResult;
      cloneUrl = cloneResult.value.effectiveUrl;
      contentDir = tmpDir;
      const shaResult = await revParse(tmpDir);
      if (!shaResult.ok) return shaResult;
      sha = shaResult.value;
    } else {
      // Non-git local dir: copy directly
      const cpResult = await wrapShell(
        () =>
          $`cp -a ${resolved.url}/. ${tmpDir}`.quiet().then(() => undefined),
        `Failed to copy local skill from "${resolved.url}"`,
        "Check that the path exists and is readable.",
      );
      if (!cpResult.ok) return cpResult;
      contentDir = tmpDir;
      sha = null;
    }
  } else {
    const cloneResult = await clone(resolved.url, tmpDir, {
      branch: effectiveRef,
      depth: 1,
    });
    if (!cloneResult.ok) return cloneResult;
    cloneUrl = cloneResult.value.effectiveUrl;
    contentDir = tmpDir;

    const shaResult = await revParse(tmpDir);
    if (!shaResult.ok) return shaResult;
    sha = shaResult.value;
  }

  // Resolve symlinks so scanner paths and contentDir match
  // (macOS: /tmp → /private/tmp; scanner resolves internally)
  contentDir = await realpath(contentDir).catch(() => contentDir);

  return ok({ contentDir, sha, cloneUrl });
}
