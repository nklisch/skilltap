import { readdir, realpath } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { debug } from "./debug";
import { parseSkillFrontmatter } from "./frontmatter";
import { SkillFrontmatterSchema } from "./schemas";
import { AGENT_PATHS } from "./symlink";

export type ScannedSkill = {
	name: string;
	description: string;
	path: string; // absolute path to skill directory
	valid: boolean;
	warnings: string[];
};

async function processSkillFile(skillMdPath: string): Promise<ScannedSkill> {
	const skillDir = dirname(skillMdPath);
	const dirName = basename(skillDir);
	const warnings: string[] = [];

	let content: string;
	try {
		content = await Bun.file(skillMdPath).text();
	} catch {
		return {
			name: dirName,
			description: "",
			path: skillDir,
			valid: false,
			warnings: ["Could not read SKILL.md"],
		};
	}

	const data = parseSkillFrontmatter(content);
	if (!data) {
		warnings.push(
			"No YAML frontmatter found — using directory name as skill name",
		);
		return {
			name: dirName,
			description: "",
			path: skillDir,
			valid: false,
			warnings,
		};
	}

	const parsed = SkillFrontmatterSchema.safeParse(data);
	if (!parsed.success) {
		const issues = parsed.error.issues.map((i) => i.message);
		return {
			name: typeof data.name === "string" && data.name ? data.name : dirName,
			description: typeof data.description === "string" ? data.description : "",
			path: skillDir,
			valid: false,
			warnings: issues,
		};
	}

	const fm = parsed.data;
	if (fm.name !== dirName) {
		warnings.push(
			`Skill name "${fm.name}" does not match directory name "${dirName}"`,
		);
	}

	return {
		name: fm.name,
		description: fm.description,
		path: skillDir,
		valid: true,
		warnings,
	};
}

// Prefer .agents/skills/ paths over agent-specific paths during deduplication.
function isAgentsSkillsPath(skillPath: string): boolean {
	return (
		skillPath.includes("/.agents/skills/") ||
		skillPath.includes("\\.agents\\skills\\")
	);
}

function deduplicate(skills: ScannedSkill[]): ScannedSkill[] {
	const byName = new Map<string, ScannedSkill>();
	for (const skill of skills) {
		const existing = byName.get(skill.name);
		if (
			!existing ||
			(isAgentsSkillsPath(skill.path) && !isAgentsSkillsPath(existing.path))
		) {
			byName.set(skill.name, skill);
		}
	}
	return Array.from(byName.values()).sort((a, b) =>
		a.name.localeCompare(b.name),
	);
}

/** List subdirectories of `parent` that contain a SKILL.md file (case-sensitive). */
async function listSkillDirs(parent: string): Promise<string[]> {
	let entries: string[];
	try {
		entries = await readdir(parent);
	} catch {
		return [];
	}
	const paths: string[] = [];
	for (const entry of entries) {
		const subDir = join(parent, entry);
		const subEntries = await readdir(subDir).catch(() => [] as string[]);
		if (subEntries.includes("SKILL.md")) {
			paths.push(join(subDir, "SKILL.md"));
		}
	}
	return paths;
}

export type ScanOptions = {
	/** Called when deep scan is needed (no skills found at standard paths). Return false to cancel. */
	onDeepScan?: (count: number) => Promise<boolean>;
};

export async function scan(dir: string, options?: ScanOptions): Promise<ScannedSkill[]> {
	// Resolve symlinks (macOS /tmp → /private/tmp) to avoid path confusion
	const resolvedDir = await realpath(dir).catch(() => dir);
	if (resolvedDir !== dir) {
		debug("scan: resolved symlink", { from: dir, to: resolvedDir });
		dir = resolvedDir;
	}
	debug("scan start", { dir });

	// Use readdir for case-sensitive filename checks throughout
	// (macOS filesystem is case-insensitive, so Bun.file("SKILL.md").exists() matches "skill.md")
	const rootEntries = await readdir(dir).catch(() => [] as string[]);

	// Step 1: Root SKILL.md — standalone or multi-skill repo with root skill
	const rootPaths: string[] = [];
	if (rootEntries.includes("SKILL.md")) {
		const rootSkillMd = join(dir, "SKILL.md");
		debug("scan: root SKILL.md found", { path: rootSkillMd });
		rootPaths.push(rootSkillMd);
	}

	// Step 2: .agents/skills/*/SKILL.md — readdir-based (avoids Bun.Glob dot-dir issues)
	const agentsPaths = await listSkillDirs(join(dir, ".agents", "skills"));
	debug("scan step 2: .agents/skills", { count: agentsPaths.length });

	// Step 2.5: skills/*/SKILL.md (antfu/skillpm convention) — priority path for npm packages
	const skillsPaths = await listSkillDirs(join(dir, "skills"));
	debug("scan step 2.5: skills/", { count: skillsPaths.length });

	// Step 3: Agent-specific paths — readdir-based (avoids Bun.Glob dot-dir issues)
	const agentSpecificPaths = (
		await Promise.all(
			Object.values(AGENT_PATHS).map((relDir) =>
				listSkillDirs(join(dir, relDir)),
			),
		)
	).flat();
	debug("scan step 3: agent-specific", { count: agentSpecificPaths.length });

	const combined = [...rootPaths, ...agentsPaths, ...skillsPaths, ...agentSpecificPaths];

	// Step 4: Deep scan fallback if nothing found
	let discoveredPaths: string[];
	if (combined.length > 0) {
		discoveredPaths = combined;
	} else {
		const deepPaths = await deepScan(dir);
		debug("scan step 4: deep scan", { count: deepPaths.length });
		if (deepPaths.length > 0 && options?.onDeepScan) {
			const proceed = await options.onDeepScan(deepPaths.length);
			if (!proceed) return [];
		}
		discoveredPaths = deepPaths;
	}

	if (discoveredPaths.length === 0) return [];

	const skills = await Promise.all(discoveredPaths.map(processSkillFile));
	const result = deduplicate(skills);
	debug("scan complete", { count: result.length, names: result.map((s) => s.name) });
	return result;
}

async function deepScan(dir: string): Promise<string[]> {
	const glob = new Bun.Glob("**/SKILL.md");
	const paths: string[] = [];
	for await (const rel of glob.scan({ cwd: dir, onlyFiles: true, dot: true })) {
		// Exclude .git internals
		if (rel.startsWith(".git/") || rel.includes("/.git/")) continue;
		paths.push(join(dir, rel));
	}
	return paths;
}
