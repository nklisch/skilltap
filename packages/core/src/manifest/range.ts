// Range parser/matcher for skilltap manifest entries.
//
// Supported range syntax:
//   "*"           — matches any ref/version (also "latest")
//   "v1.2.3"      — exact match (any non-semver string is treated as exact)
//   "^1.0"        — caret: same major version, >= 1.0.0
//   "^1.2.3"      — caret: same major version, >= 1.2.3
//   "~1.2"        — tilde: same minor, >= 1.2.0
//   "~1.2.3"      — tilde: same minor, >= 1.2.3
//
// Non-semver candidates (branch names, sha-likes) only match an exact
// range. The matcher gracefully returns false rather than throwing on
// non-comparable inputs.

export type ParsedRange =
  | { kind: "any" }
  | { kind: "exact"; value: string }
  | { kind: "caret"; major: number; minor: number; patch: number }
  | { kind: "tilde"; major: number; minor: number; patch: number };

interface SemVer {
  major: number;
  minor: number;
  patch: number;
  prerelease?: string;
}

const SEMVER_RE = /^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([0-9A-Za-z-.]+))?$/;

function parseSemver(input: string): SemVer | null {
  const match = SEMVER_RE.exec(input.trim());
  if (!match) return null;
  const major = Number.parseInt(match[1] ?? "", 10);
  if (Number.isNaN(major)) return null;
  const minor = match[2] === undefined ? 0 : Number.parseInt(match[2], 10);
  const patch = match[3] === undefined ? 0 : Number.parseInt(match[3], 10);
  if (Number.isNaN(minor) || Number.isNaN(patch)) return null;
  return {
    major,
    minor,
    patch,
    prerelease: match[4],
  };
}

function compareSemver(a: SemVer, b: SemVer): number {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  if (a.patch !== b.patch) return a.patch - b.patch;
  // Prereleases sort lower than their non-prerelease counterpart.
  if (a.prerelease && !b.prerelease) return -1;
  if (!a.prerelease && b.prerelease) return 1;
  if (a.prerelease && b.prerelease) {
    return a.prerelease < b.prerelease ? -1 : a.prerelease > b.prerelease ? 1 : 0;
  }
  return 0;
}

export function parseRange(input: string): ParsedRange {
  const trimmed = input.trim();
  if (trimmed === "*" || trimmed === "" || trimmed === "latest") {
    return { kind: "any" };
  }
  if (trimmed.startsWith("^")) {
    const version = parseSemver(trimmed.slice(1));
    if (version === null) {
      return { kind: "exact", value: trimmed };
    }
    return {
      kind: "caret",
      major: version.major,
      minor: version.minor,
      patch: version.patch,
    };
  }
  if (trimmed.startsWith("~")) {
    const version = parseSemver(trimmed.slice(1));
    if (version === null) {
      return { kind: "exact", value: trimmed };
    }
    return {
      kind: "tilde",
      major: version.major,
      minor: version.minor,
      patch: version.patch,
    };
  }
  return { kind: "exact", value: trimmed };
}

export function matchesRange(range: ParsedRange, candidate: string): boolean {
  if (range.kind === "any") return true;
  if (range.kind === "exact") {
    return candidate.trim() === range.value;
  }
  const version = parseSemver(candidate);
  if (version === null) return false;
  if (range.kind === "caret") {
    if (version.major !== range.major) return false;
    return compareSemver(version, {
      major: range.major,
      minor: range.minor,
      patch: range.patch,
    }) >= 0;
  }
  // tilde
  if (version.major !== range.major) return false;
  if (version.minor !== range.minor) return false;
  return compareSemver(version, {
    major: range.major,
    minor: range.minor,
    patch: range.patch,
  }) >= 0;
}

// Given a parsed range and a list of candidates, return the highest matching
// candidate (semver-sorted). For non-semver candidates only the exact match
// applies. Returns null if no candidate matches.
export function findBestMatch(range: ParsedRange, candidates: string[]): string | null {
  if (range.kind === "exact") {
    return candidates.find((c) => c.trim() === range.value) ?? null;
  }
  const matching = candidates.filter((c) => matchesRange(range, c));
  if (matching.length === 0) return null;
  // Sort matching candidates by semver descending. Non-semver entries sort
  // lower than any semver entry.
  matching.sort((a, b) => {
    const av = parseSemver(a);
    const bv = parseSemver(b);
    if (av && bv) return compareSemver(bv, av);
    if (av && !bv) return -1;
    if (!av && bv) return 1;
    return 0;
  });
  return matching[0] ?? null;
}
