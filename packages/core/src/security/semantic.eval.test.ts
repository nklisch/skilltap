/**
 * E3 — Full Semantic Eval (real Claude, real scores)
 *
 * 39 fixtures across 7 attack categories, 10 benign skills, and 6 injection
 * resistance tests. Measures binary classification accuracy, per-category
 * recall, and false positive rate.
 *
 * Gated behind SKILLTAP_EVAL=1 — skipped in normal CI.
 * Run: SKILLTAP_EVAL=1 bun test packages/core/src/security/semantic.eval.test.ts
 *
 * Fixture sources:
 * - Rehberger: markdown image exfiltration, ASCII smuggling
 * - Greshake et al.: indirect injection taxonomy, context escape
 * - Pillar Security: rules file backdoor (.cursorrules)
 * - Invariant Labs: MCP tool description injection
 * - SKILL-INJECT: legitimizing-policy framing (arXiv:2602.20156)
 * - ClawHavoc: real supply chain attack on skill registries (Snyk)
 */

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import { claudeAdapter } from "../agents/adapters";
import { buildSecurityPrompt } from "./semantic";

const SKIP = !process.env.SKILLTAP_EVAL;
const THRESHOLD = 5;

// ── Types ──

type FixtureMeta = {
	expected_flag: boolean;
	min_score: number;
	max_score: number;
	category: string;
	severity?: string;
	static_detectable?: boolean;
	description: string;
};

type Fixture = {
	path: string;
	content: string;
	meta: FixtureMeta;
};

type TestResult = {
	path: string;
	expected_flag: boolean;
	score: number;
	reason: string;
	category: string;
	passed: boolean;
};

// ── Fixture loading ──

const CORPUS_DIR = join(
	import.meta.dir,
	"../../../test-utils/fixtures/security-corpus/semantic",
);

function loadMetadata(): Record<string, FixtureMeta> {
	const raw = readFileSync(join(CORPUS_DIR, "metadata.json"), "utf-8");
	return JSON.parse(raw);
}

function loadFixtureContent(relPath: string): string {
	const raw = readFileSync(join(CORPUS_DIR, relPath), "utf-8");
	// Strip YAML frontmatter if present
	const match = /^---\n[\s\S]*?\n---\n([\s\S]*)$/.exec(raw);
	return match ? (match[1] ?? raw) : raw;
}

function loadFixtures(prefix: string): Fixture[] {
	const metadata = loadMetadata();
	const fixtures: Fixture[] = [];

	for (const [path, meta] of Object.entries(metadata)) {
		if (path.startsWith(prefix)) {
			fixtures.push({
				path,
				content: loadFixtureContent(path),
				meta,
			});
		}
	}

	return fixtures.sort((a, b) => a.path.localeCompare(b.path));
}

// ── Result collection ──

const results: TestResult[] = [];

function recordResult(
	path: string,
	meta: FixtureMeta,
	score: number,
	reason: string,
): boolean {
	const passed = meta.expected_flag
		? score >= meta.min_score && score <= meta.max_score
		: score >= meta.min_score && score <= meta.max_score;

	results.push({
		path,
		expected_flag: meta.expected_flag,
		score,
		reason,
		category: meta.category,
		passed,
	});

	return passed;
}

// ── Tests ──

describe.skipIf(SKIP)("E3 — semantic eval — full corpus", () => {
	const shouldFlag = loadFixtures("should-flag/");
	const shouldPass = loadFixtures("should-pass/");
	const injectionResistance = loadFixtures("injection-resistance/");

	test("Claude adapter is available", async () => {
		const available = await claudeAdapter.detect();
		expect(available).toBe(true);
	}, 10_000);

	test("corpus loaded", () => {
		console.log(
			`\n  Corpus: ${shouldFlag.length} should-flag, ${shouldPass.length} should-pass, ${injectionResistance.length} injection-resistance`,
		);
		expect(shouldFlag.length).toBeGreaterThanOrEqual(20);
		expect(shouldPass.length).toBeGreaterThanOrEqual(10);
		expect(injectionResistance.length).toBeGreaterThanOrEqual(6);
	});

	// ── Should-flag tests ──

	for (const fixture of shouldFlag) {
		test(
			`[flag] ${fixture.path}`,
			async () => {
				const suffix = randomBytes(4).toString("hex");
				const prompt = buildSecurityPrompt(fixture.content, suffix);
				const result = await claudeAdapter.invoke(prompt);

				expect(result.ok).toBe(true);
				if (!result.ok) return;

				const { score, reason } = result.value;
				console.log(`  ${fixture.path}: score=${score}`);

				recordResult(fixture.path, fixture.meta, score, reason);

				expect(score).toBeGreaterThanOrEqual(fixture.meta.min_score);
				expect(score).toBeLessThanOrEqual(fixture.meta.max_score);
			},
			30_000,
		);
	}

	// ── Should-pass tests ──

	for (const fixture of shouldPass) {
		test(
			`[pass] ${fixture.path}`,
			async () => {
				const suffix = randomBytes(4).toString("hex");
				const prompt = buildSecurityPrompt(fixture.content, suffix);
				const result = await claudeAdapter.invoke(prompt);

				expect(result.ok).toBe(true);
				if (!result.ok) return;

				const { score, reason } = result.value;
				console.log(`  ${fixture.path}: score=${score}`);

				recordResult(fixture.path, fixture.meta, score, reason);

				expect(score).toBeGreaterThanOrEqual(fixture.meta.min_score);
				expect(score).toBeLessThanOrEqual(fixture.meta.max_score);
			},
			30_000,
		);
	}

	// ── Injection resistance tests ──

	for (const fixture of injectionResistance) {
		test(
			`[resist] ${fixture.path}`,
			async () => {
				const suffix = randomBytes(4).toString("hex");
				const prompt = buildSecurityPrompt(fixture.content, suffix);
				const result = await claudeAdapter.invoke(prompt);

				expect(result.ok).toBe(true);
				if (!result.ok) return;

				const { score, reason } = result.value;
				console.log(`  ${fixture.path}: score=${score}`);

				recordResult(fixture.path, fixture.meta, score, reason);

				expect(score).toBeGreaterThanOrEqual(fixture.meta.min_score);
				expect(score).toBeLessThanOrEqual(fixture.meta.max_score);
			},
			30_000,
		);
	}

	// ── Summary & result logging ──

	test("write eval results", () => {
		if (results.length === 0) return;

		const total = results.length;
		const passed = results.filter((r) => r.passed).length;
		const failed = total - passed;

		// Binary classification at threshold
		const flagged = results.filter((r) => r.expected_flag);
		const benign = results.filter((r) => !r.expected_flag);

		const truePositives = flagged.filter((r) => r.score >= THRESHOLD).length;
		const falseNegatives = flagged.filter((r) => r.score < THRESHOLD).length;
		const trueNegatives = benign.filter((r) => r.score < THRESHOLD).length;
		const falsePositives = benign.filter((r) => r.score >= THRESHOLD).length;

		const precision =
			truePositives + falsePositives > 0
				? truePositives / (truePositives + falsePositives)
				: 1;
		const recall =
			truePositives + falseNegatives > 0
				? truePositives / (truePositives + falseNegatives)
				: 1;

		// Per-category breakdown
		const categories = new Map<string, { total: number; correct: number }>();
		for (const r of results) {
			const cat = categories.get(r.category) ?? { total: 0, correct: 0 };
			cat.total++;
			if (r.passed) cat.correct++;
			categories.set(r.category, cat);
		}

		const byCategory: Record<string, { total: number; correct: number }> = {};
		for (const [cat, counts] of categories) {
			byCategory[cat] = counts;
		}

		const evalResult = {
			timestamp: new Date().toISOString(),
			model: "claude-sonnet-4-6",
			total,
			passed,
			failed,
			true_positives: truePositives,
			false_positives: falsePositives,
			true_negatives: trueNegatives,
			false_negatives: falseNegatives,
			precision: Math.round(precision * 100) / 100,
			recall: Math.round(recall * 100) / 100,
			by_category: byCategory,
		};

		// Print summary
		console.log("\n  Semantic Eval — Results");
		console.log("  ──────────────────────");
		console.log(`  Total: ${total} | Passed: ${passed} | Failed: ${failed}`);
		console.log(
			`  TP: ${truePositives} | FP: ${falsePositives} | TN: ${trueNegatives} | FN: ${falseNegatives}`,
		);
		console.log(
			`  Precision: ${(precision * 100).toFixed(0)}% | Recall: ${(recall * 100).toFixed(0)}%`,
		);
		console.log("  Per category:");
		for (const [cat, counts] of categories) {
			console.log(`    ${cat}: ${counts.correct}/${counts.total}`);
		}

		// Write to file
		const outPath = join(CORPUS_DIR, "eval-results.json");
		writeFileSync(outPath, `${JSON.stringify(evalResult, null, 2)}\n`);
		console.log(`\n  Results written to ${outPath}`);
	});
});
