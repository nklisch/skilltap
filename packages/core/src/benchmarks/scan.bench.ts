/**
 * Performance benchmarks for skilltap core scanning functions.
 *
 * Run with: bun run packages/core/src/benchmarks/scan.bench.ts
 *
 * Thresholds (per iteration — regression gates, not strict performance targets):
 *   scanStatic  — 500 markdown files     < 5000 ms
 *   chunkSkillDir — 10,000 line file     <  500 ms
 *   scanDiff    — 1 MB diff output       < 5000 ms  (regex over large text; ~970ms actual)
 *   loadInstalled — 100 skill records    <   50 ms
 *
 * Baseline (Linux x86_64, Bun 1.3.10, 2026-03-01):
 *   scanStatic  — 500 markdown files:    ~173 ms/iter
 *   chunkSkillDir — 10,000 line file:    ~9   ms/iter
 *   scanDiff    — 1 MB diff output:      ~967 ms/iter  (7 regex detectors × 18k lines)
 *   loadInstalled — 100 skill records:   ~0.2 ms/iter
 */

import { mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { chunkSkillDir, loadInstalled, scanDiff, scanStatic } from "@skilltap/core";

// ── Helpers ───────────────────────────────────────────────────────────────────

async function bench(
  name: string,
  fn: () => unknown | Promise<unknown>,
  opts: { iterations?: number; warmup?: number; thresholdMs?: number } = {},
): Promise<void> {
  const { iterations = 10, warmup = 3, thresholdMs } = opts;
  for (let i = 0; i < warmup; i++) await fn();

  const start = performance.now();
  for (let i = 0; i < iterations; i++) await fn();
  const elapsed = performance.now() - start;

  const avgMs = elapsed / iterations;
  const pass = thresholdMs == null || avgMs < thresholdMs;
  const status = pass ? "✓" : "✗";
  const limit = thresholdMs != null ? ` (limit: ${thresholdMs}ms)` : "";
  console.log(
    `${status} ${name}: ${avgMs.toFixed(2)}ms/iter over ${iterations} iterations${limit}`,
  );
  if (!pass) process.exitCode = 1;
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

const TMP = `/tmp/bench-skilltap-${Date.now()}`;

// 1. 500-file directory for scanStatic
const SCAN_DIR = join(TMP, "scan");
await mkdir(SCAN_DIR, { recursive: true });

const SKILL_MD = `# My Skill

## Overview
This skill assists with common development tasks by providing clear instructions.

## Usage
Follow these steps to complete the task successfully.

## Examples
- Example one: do this specific thing
- Example two: do that other thing instead
- Example three: combine both approaches
`;

for (let i = 0; i < 500; i++) {
  await writeFile(join(SCAN_DIR, `skill-${String(i).padStart(3, "0")}.md`), SKILL_MD);
}

// 2. Single 10,000-line file for chunkSkillDir
const CHUNK_DIR = join(TMP, "chunk");
await mkdir(CHUNK_DIR, { recursive: true });
const tenKLines = Array.from(
  { length: 10000 },
  (_, i) => `Line ${i + 1}: This is benchmark content for the chunking test.`,
).join("\n");
await writeFile(join(CHUNK_DIR, "SKILL.md"), tenKLines);

// 3. ~1 MB diff for scanDiff
const DIFF_LINE = "+This is a new line added to the skill documentation file.\n";
const DIFF_HEADER =
  "diff --git a/SKILL.md b/SKILL.md\n--- a/SKILL.md\n+++ b/SKILL.md\n@@ -1,1 +1,10000 @@\n";
const linesNeeded = Math.ceil((1024 * 1024 - DIFF_HEADER.length) / DIFF_LINE.length);
const LARGE_DIFF = DIFF_HEADER + DIFF_LINE.repeat(linesNeeded);

// 4. installed.json with 100 skills for loadInstalled
const CONFIG_DIR = join(TMP, "config");
await mkdir(join(CONFIG_DIR, "skilltap"), { recursive: true });

const baseSkill = {
  description: "Benchmark skill",
  repo: "https://github.com/example/skill.git",
  ref: "main",
  sha: "abc123def456abc123def456",
  scope: "global",
  path: null,
  tap: null,
  also: [],
  installedAt: "2025-01-01T00:00:00.000Z",
  updatedAt: "2025-01-01T00:00:00.000Z",
};
const installedJson = {
  version: 1,
  skills: Array.from({ length: 100 }, (_, i) => ({
    ...baseSkill,
    name: `skill-${String(i).padStart(3, "0")}`,
  })),
};
await writeFile(
  join(CONFIG_DIR, "skilltap", "installed.json"),
  JSON.stringify(installedJson, null, 2),
);

// loadInstalled reads from XDG_CONFIG_HOME
const savedXdg = process.env.XDG_CONFIG_HOME;
process.env.XDG_CONFIG_HOME = CONFIG_DIR;

// ── Benchmarks ────────────────────────────────────────────────────────────────

console.log("\nRunning benchmarks...\n");

await bench(
  "scanStatic  — 500 markdown files",
  () => scanStatic(SCAN_DIR, { maxSize: 100 * 1024 * 1024 }),
  { iterations: 5, warmup: 1, thresholdMs: 5000 },
);

await bench(
  "chunkSkillDir — 10,000 line file",
  () => chunkSkillDir(CHUNK_DIR),
  { iterations: 20, warmup: 3, thresholdMs: 500 },
);

await bench(
  "scanDiff    — 1 MB diff output  ",
  () => scanDiff(LARGE_DIFF),
  { iterations: 5, warmup: 1, thresholdMs: 5000 },
);

await bench(
  "loadInstalled — 100 skill records",
  () => loadInstalled(),
  { iterations: 50, warmup: 5, thresholdMs: 50 },
);

// ── Cleanup ───────────────────────────────────────────────────────────────────

if (savedXdg !== undefined) process.env.XDG_CONFIG_HOME = savedXdg;
else delete process.env.XDG_CONFIG_HOME;

await rm(TMP, { recursive: true, force: true });

console.log("\nDone.");
