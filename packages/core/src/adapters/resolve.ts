import type { ResolvedSource } from "../schemas";
import type { Result } from "../types";
import { err, UserError } from "../types";
import { gitAdapter } from "./git";
import { githubAdapter } from "./github";
import { httpAdapter } from "./http";
import { localAdapter } from "./local";
import { npmAdapter } from "./npm";
import type { SourceAdapter } from "./types";

// Resolution order per SPEC:
// 1. URL protocols (https://, http://, git@, ssh://) → git
// 2. npm: prefix → npm
// 3. url: prefix → http (direct tarball from HTTP registry)
// 4. Local paths (./, /, ~/) → local
// 5. Bare owner/repo → github (shorthand)
const ADAPTERS: SourceAdapter[] = [gitAdapter, npmAdapter, httpAdapter, localAdapter, githubAdapter];

export async function resolveSource(
  source: string,
): Promise<Result<ResolvedSource, UserError>> {
  for (const adapter of ADAPTERS) {
    if (adapter.canHandle(source)) return adapter.resolve(source);
  }
  return err(
    new UserError(
      `Cannot resolve source: "${source}"`,
      `Try a full URL, GitHub shorthand (user/repo), local path (./path), or a skill name from a configured tap`,
    ),
  );
}
