import type { ResolvedSource } from "../schemas";
import type { Result } from "../types";
import { err, UserError } from "../types";
import { gitAdapter } from "./git";
import { createGithubAdapter } from "./github";
import { httpAdapter } from "./http";
import { localAdapter } from "./local";
import { npmAdapter } from "./npm";
import type { SourceAdapter } from "./types";

const DEFAULT_GIT_HOST = "https://github.com";

export async function resolveSource(
  source: string,
  gitHost?: string,
): Promise<Result<ResolvedSource, UserError>> {
  const host = gitHost ?? DEFAULT_GIT_HOST;
  const adapters: SourceAdapter[] = [
    gitAdapter,
    npmAdapter,
    httpAdapter,
    localAdapter,
    createGithubAdapter(host),
  ];

  for (const adapter of adapters) {
    if (adapter.canHandle(source)) return adapter.resolve(source);
  }
  return err(
    new UserError(
      `Cannot resolve source: "${source}"`,
      `Try a full URL, GitHub shorthand (user/repo), local path (./path), or a skill name from a configured tap`,
    ),
  );
}
