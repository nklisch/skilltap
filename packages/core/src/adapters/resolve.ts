import { err } from "../types"
import { UserError } from "../types"
import type { Result } from "../types"
import type { ResolvedSource } from "../schemas"
import { gitAdapter } from "./git"
import { localAdapter } from "./local"
import { githubAdapter } from "./github"
import type { SourceAdapter } from "./types"

const ADAPTERS: SourceAdapter[] = [gitAdapter, localAdapter, githubAdapter]

export async function resolveSource(
  source: string,
): Promise<Result<ResolvedSource, UserError>> {
  for (const adapter of ADAPTERS) {
    if (adapter.canHandle(source)) return adapter.resolve(source)
  }
  // TODO Phase 7: tap@ref resolution and tap name search
  return err(
    new UserError(
      `Cannot resolve source: "${source}"`,
      `Try a full URL, GitHub shorthand (user/repo), or a local path (./path)`,
    ),
  )
}
