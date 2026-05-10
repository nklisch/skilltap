import { loadTaps } from "../taps";
import type { TapEntry } from "../taps";
import type { Result } from "../types";
import { err, ok, UserError } from "../types";
import type { InstallOptions } from "./types";

function looksLikeTapName(source: string): boolean {
  if (
    source.startsWith("./") ||
    source.startsWith("/") ||
    source.startsWith("~/")
  )
    return false;
  if (/^(https?:\/\/|git@|ssh:\/\/|github:|npm:)/.test(source)) return false;
  const name = source.includes("@")
    ? source.slice(0, source.lastIndexOf("@"))
    : source;
  if (name.includes("/")) return false;
  return true;
}

export function parseTapPluginRef(
  source: string,
): { tapName: string; pluginName: string } | null {
  if (!source.includes("/")) return null;
  const parts = source.split("/");
  if (parts.length !== 2) return null;
  if (/^(https?:\/\/|git@|ssh:\/\/|github:|npm:)/.test(source)) return null;
  if (
    source.startsWith("./") ||
    source.startsWith("/") ||
    source.startsWith("~/")
  )
    return null;
  // biome-ignore lint/style/noNonNullAssertion: parts.length === 2 guard above
  return { tapName: parts[0]!, pluginName: parts[1]! };
}

export type TapResolution = {
  source: string;
  tap: string;
  skillName: string;
  ref?: string;
};

/** If source looks like a tap name (or name@ref), resolve it via configured taps. Returns null if not a tap name. */
export async function resolveTapName(
  source: string,
  ref: string | undefined,
  onSelectTap?: InstallOptions["onSelectTap"],
): Promise<Result<TapResolution | null, UserError>> {
  if (!looksLikeTapName(source)) return ok(null);

  let tapName = source;
  let effectiveRef = ref;
  if (source.includes("@")) {
    const atIdx = source.lastIndexOf("@");
    tapName = source.slice(0, atIdx);
    if (!effectiveRef) effectiveRef = source.slice(atIdx + 1);
  }

  const tapsResult = await loadTaps();
  if (!tapsResult.ok) return tapsResult;
  const allSkills = tapsResult.value;

  if (allSkills.length === 0) {
    return err(
      new UserError(
        `No taps configured. Add one with 'skilltap tap add <name> <url>'.`,
      ),
    );
  }

  const matches = allSkills.filter((e) => e.skill.name === tapName);
  if (matches.length === 0) {
    return err(
      new UserError(
        `Skill '${tapName}' not found in any configured tap.`,
        `Run 'skilltap find ${tapName}' to search, or check tap names with 'skilltap tap list'`,
      ),
    );
  }

  let chosen: TapEntry;
  if (matches.length === 1) {
    // biome-ignore lint/style/noNonNullAssertion: matches.length === 1 guarantees index 0 exists
    chosen = matches[0]!;
  } else if (onSelectTap) {
    const selected = await onSelectTap(matches);
    if (!selected) return err(new UserError("Install cancelled."));
    chosen = selected;
  } else {
    // biome-ignore lint/style/noNonNullAssertion: matches.length > 0 guaranteed (checked above)
    chosen = matches[0]!;
  }

  return ok({
    source: chosen.skill.repo,
    tap: chosen.tapName,
    skillName: tapName,
    ref: effectiveRef,
  });
}
