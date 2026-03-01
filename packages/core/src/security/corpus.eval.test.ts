/**
 * E1 — Static Detector Precision/Recall Tests
 *
 * Tests the 7 regex-based detectors against a labeled corpus of fixtures.
 * Measures recall (does each detector fire on true-positive fixtures?)
 * and precision (does each detector stay silent on true-negative fixtures?).
 *
 * No LLM, no network, no cost. Runs with `bun test`.
 */

import { readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import {
	detectDangerousPatterns,
	detectHiddenHtmlCss,
	detectInvisibleUnicode,
	detectMarkdownHiding,
	detectObfuscation,
	detectSuspiciousUrls,
	detectTagInjection,
} from "./patterns";

// ── Detector registry ──

const DETECTORS: Record<string, (content: string) => unknown[]> = {
	detectInvisibleUnicode,
	detectHiddenHtmlCss,
	detectMarkdownHiding,
	detectObfuscation,
	detectSuspiciousUrls,
	detectDangerousPatterns,
	detectTagInjection,
};

const ALL_DETECTOR_FNS = Object.values(DETECTORS);

// ── Fixture loading ──

type FixtureMeta = {
	expected_detectors: string[];
	expected_categories: string[];
	expected_min_count: number;
	label: string;
	fires?: boolean;
	description: string;
};

type Fixture = {
	name: string;
	relPath: string;
	content: string;
	meta: FixtureMeta;
};

const CORPUS_DIR = join(
	import.meta.dir,
	"../../../test-utils/fixtures/security-corpus/static",
);

function parseFrontmatter(raw: string): { meta: Record<string, unknown>; content: string } {
	const match = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/.exec(raw);
	if (!match) return { meta: {}, content: raw };

	const yamlBlock = match[1] ?? "";
	const content = match[2] ?? "";
	const meta: Record<string, unknown> = {};

	for (const line of yamlBlock.split("\n")) {
		const colonIdx = line.indexOf(":");
		if (colonIdx === -1) continue;
		const key = line.slice(0, colonIdx).trim();
		let value: string | unknown = line.slice(colonIdx + 1).trim();

		// Parse arrays: ["a", "b"]
		if (typeof value === "string" && value.startsWith("[")) {
			try {
				value = JSON.parse(value);
			} catch {
				// keep as string
			}
		}
		// Parse numbers
		else if (typeof value === "string" && /^\d+$/.test(value)) {
			value = parseInt(value, 10);
		}
		// Parse booleans
		else if (value === "true") value = true;
		else if (value === "false") value = false;
		// Strip quotes from strings
		else if (
			typeof value === "string" &&
			value.startsWith('"') &&
			value.endsWith('"')
		) {
			value = value.slice(1, -1);
		}

		meta[key] = value;
	}

	return { meta, content };
}

function loadFixtures(subdir: string): Fixture[] {
	const dir = join(CORPUS_DIR, subdir);
	const fixtures: Fixture[] = [];

	function walk(currentDir: string, prefix: string) {
		for (const entry of readdirSync(currentDir, { withFileTypes: true })) {
			if (entry.isDirectory()) {
				walk(join(currentDir, entry.name), `${prefix}${entry.name}/`);
			} else if (entry.name.endsWith(".md")) {
				const filePath = join(currentDir, entry.name);
				const raw = require("node:fs").readFileSync(filePath, "utf-8");
				const { meta, content } = parseFrontmatter(raw);
				fixtures.push({
					name: `${prefix}${entry.name}`,
					relPath: `${prefix}${entry.name}`,
					content,
					meta: {
						expected_detectors: (meta.expected_detectors as string[]) ?? [],
						expected_categories: (meta.expected_categories as string[]) ?? [],
						expected_min_count: (meta.expected_min_count as number) ?? 0,
						label: (meta.label as string) ?? subdir,
						fires: meta.fires as boolean | undefined,
						description: (meta.description as string) ?? "",
					},
				});
			}
		}
	}

	walk(dir, "");
	return fixtures;
}

// ── Load all fixtures ──

const truePositives = loadFixtures("true-positives");
const trueNegatives = loadFixtures("true-negatives");
const boundary = loadFixtures("boundary");

// ── Tests ──

describe("E1 — static detector recall (true positives)", () => {
	for (const fixture of truePositives) {
		test(`catches: ${fixture.name}`, () => {
			let totalMatches = 0;
			const matchedCategories: string[] = [];

			for (const detectorName of fixture.meta.expected_detectors) {
				const detector = DETECTORS[detectorName];
				expect(detector).toBeDefined();
				if (!detector) continue;

				const matches = detector(fixture.content) as Array<{
					category: string;
				}>;
				totalMatches += matches.length;
				for (const m of matches) {
					matchedCategories.push(m.category);
				}
			}

			expect(totalMatches).toBeGreaterThanOrEqual(
				fixture.meta.expected_min_count,
			);

			// Verify at least one expected category was found
			for (const expectedCat of fixture.meta.expected_categories) {
				expect(matchedCategories).toContain(expectedCat);
			}
		});
	}
});

describe("E1 — static detector precision (true negatives)", () => {
	for (const fixture of trueNegatives) {
		test(`no false positives: ${fixture.name}`, () => {
			for (const detector of ALL_DETECTOR_FNS) {
				const matches = detector(fixture.content);
				expect(matches).toEqual([]);
			}
		});
	}
});

describe("E1 — boundary cases", () => {
	for (const fixture of boundary) {
		const shouldFire = fixture.meta.fires !== false;
		const label = shouldFire ? "fires (accepted)" : "silent";

		test(`[${label}] ${fixture.name}`, () => {
			if (shouldFire) {
				// At least one expected detector should fire
				let totalMatches = 0;
				for (const detectorName of fixture.meta.expected_detectors) {
					const detector = DETECTORS[detectorName];
					if (!detector) continue;
					const matches = detector(fixture.content);
					totalMatches += matches.length;
				}
				expect(totalMatches).toBeGreaterThanOrEqual(
					fixture.meta.expected_min_count,
				);
			} else {
				// No detector should fire
				for (const detector of ALL_DETECTOR_FNS) {
					const matches = detector(fixture.content);
					expect(matches).toEqual([]);
				}
			}
		});
	}
});

// ── Summary ──

describe("E1 — corpus summary", () => {
	test("reports fixture counts", () => {
		console.log("\n  Static Corpus Summary");
		console.log("  ─────────────────────");
		console.log(`  True positives: ${truePositives.length}`);
		console.log(`  True negatives: ${trueNegatives.length}`);
		console.log(`  Boundary cases: ${boundary.length}`);
		console.log(`  Total fixtures: ${truePositives.length + trueNegatives.length + boundary.length}`);

		expect(truePositives.length).toBeGreaterThanOrEqual(20);
		expect(trueNegatives.length).toBeGreaterThanOrEqual(10);
		expect(boundary.length).toBeGreaterThanOrEqual(5);
	});
});
