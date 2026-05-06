import type { SourceForPolicy } from "./types";

// Escape regex specials, preserve `*` as a multi-char wildcard.
function patternToRegex(pattern: string): RegExp {
  const escaped = pattern
    .replace(/[.+?^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*");
  return new RegExp(`^${escaped}$`);
}

// True iff `target` exactly matches `pattern` under glob semantics.
// Anchored at both ends; only `*` is special.
export function trustMatches(pattern: string, target: string): boolean {
  return patternToRegex(pattern).test(target);
}

// True iff any pattern in `trust` matches the source's tap name OR full URL.
// Empty trust list → false. Empty target string → checked but rarely matches.
export function isTrusted(trust: string[], source: SourceForPolicy): boolean {
  for (const pattern of trust) {
    if (source.tapName && trustMatches(pattern, source.tapName)) return true;
    if (trustMatches(pattern, source.sourceUrl)) return true;
  }
  return false;
}
