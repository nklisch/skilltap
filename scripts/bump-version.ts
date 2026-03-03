#!/usr/bin/env bun
/**
 * Usage:  bun scripts/bump-version.ts <patch|minor|major|x.y.z>
 *
 * Bumps the version in packages/core/package.json and packages/cli/package.json
 * in lockstep, commits, and tags. core/package.json is the source of truth
 * read by VERSION at runtime.
 */
import { $ } from "bun";

const PACKAGES = [
  "packages/core/package.json",
  "packages/cli/package.json",
];

const arg = process.argv[2];
if (!arg) {
  console.error("Usage: bun scripts/bump-version.ts <patch|minor|major|x.y.z>");
  process.exit(1);
}

// Read current version from core (source of truth)
const core = await Bun.file(PACKAGES[0]!).json() as { version: string };
const [major, minor, patch] = core.version.split(".").map(Number) as [number, number, number];

let next: string;
if (arg === "patch") {
  next = `${major}.${minor}.${patch + 1}`;
} else if (arg === "minor") {
  next = `${major}.${minor + 1}.0`;
} else if (arg === "major") {
  next = `${major + 1}.0.0`;
} else if (/^\d+\.\d+\.\d+$/.test(arg)) {
  next = arg;
} else {
  console.error(`Unknown version argument: ${arg}`);
  process.exit(1);
}

console.log(`${core.version} → ${next}`);

for (const path of PACKAGES) {
  const pkg = await Bun.file(path).json() as Record<string, unknown>;
  pkg.version = next;
  await Bun.write(path, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`  updated ${path}`);
}

// Commit, tag, and push
const tag = `v${next}`;
await $`git commit -am "Release ${tag}"`;
await $`git tag ${tag}`;
await $`git push`;
await $`git push origin ${tag}`;
console.log(`\nReleased ${tag}.`);
