#!/usr/bin/env bun
/**
 * Usage:  bun scripts/bump-version.ts <patch|minor|major|x.y.z[-prerelease]>
 *
 * Bumps the version in packages/core/package.json and packages/cli/package.json
 * in lockstep, commits, and tags. core/package.json is the source of truth
 * read by VERSION at runtime.
 *
 * Exact versions accept an optional semver prerelease tag (e.g. 2.0.0-rc.1,
 * 1.5.0-beta, 0.9.0-alpha.2). The patch/minor/major shortcuts always produce
 * a clean MAJOR.MINOR.PATCH release.
 *
 * Environment:
 *   SKILLTAP_BUMP_NO_PUSH=1   Stage the commit + tag locally but skip
 *                             `git push`. The user reviews and pushes
 *                             manually. Useful for autopilot runs that
 *                             must not push to remote.
 */
import { $ } from "bun";

const PACKAGES = [
  "packages/core/package.json",
  "packages/cli/package.json",
];

const arg = process.argv[2];
if (!arg) {
  console.error(
    "Usage: bun scripts/bump-version.ts <patch|minor|major|x.y.z[-prerelease]>",
  );
  process.exit(1);
}

// Read current version from core (source of truth). Strip any prerelease tag
// before parsing the numeric core, so 'patch' on 2.0.0-rc.1 still works
// (yielding 2.0.1 — drop prerelease, bump patch).
const core = (await Bun.file(PACKAGES[0]!).json()) as { version: string };
const versionCore = core.version.split("-")[0]!;
const [major, minor, patch] = versionCore.split(".").map(Number) as [
  number,
  number,
  number,
];

// Permissive semver: MAJOR.MINOR.PATCH with optional prerelease (-tag, -tag.N,
// -tag.N.M etc). Build metadata (+build) intentionally not supported — it's
// rare and complicates the tag/commit story.
const SEMVER_RE = /^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$/;

let next: string;
if (arg === "patch") {
  next = `${major}.${minor}.${patch + 1}`;
} else if (arg === "minor") {
  next = `${major}.${minor + 1}.0`;
} else if (arg === "major") {
  next = `${major + 1}.0.0`;
} else if (SEMVER_RE.test(arg)) {
  next = arg;
} else {
  console.error(`Unknown version argument: ${arg}`);
  process.exit(1);
}

console.log(`${core.version} → ${next}`);

for (const path of PACKAGES) {
  const pkg = (await Bun.file(path).json()) as Record<string, unknown>;
  pkg.version = next;
  await Bun.write(path, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`  updated ${path}`);
}

// Commit and tag. Push is gated on SKILLTAP_BUMP_NO_PUSH so autopilot runs
// (which must not push to remote) can stage the bump for the user to review.
const tag = `v${next}`;
await $`git commit -am "Release ${tag}"`;
await $`git tag ${tag}`;

if (process.env.SKILLTAP_BUMP_NO_PUSH === "1") {
  console.log(
    `\nStaged ${tag} locally (commit + tag). SKILLTAP_BUMP_NO_PUSH=1 — push skipped.\n` +
      `  Run when ready: git push --follow-tags`,
  );
} else {
  await $`git push`;
  await $`git push origin ${tag}`;
  console.log(`\nReleased ${tag}.`);
}
