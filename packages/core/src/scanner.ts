import { basename, dirname, join } from "node:path";
import { SkillFrontmatterSchema } from "./schemas";

export type ScannedSkill = {
  name: string;
  description: string;
  path: string; // absolute path to skill directory
  valid: boolean;
  warnings: string[];
};

// Parse YAML-style frontmatter between leading --- delimiters.
// Returns null if no frontmatter found.
function parseSkillFrontmatter(
  content: string,
): Record<string, unknown> | null {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return null;
  // biome-ignore lint/style/noNonNullAssertion: match[1] is defined because the regex has a capturing group
  const block = match[1]!;
  const data: Record<string, unknown> = {};
  for (const line of block.split("\n")) {
    const sep = line.indexOf(":");
    if (sep === -1) continue;
    const key = line.slice(0, sep).trim();
    if (!key) continue;
    const raw = line.slice(sep + 1).trim();
    if (raw === "true") data[key] = true;
    else if (raw === "false") data[key] = false;
    else if (raw !== "" && !Number.isNaN(Number(raw))) data[key] = Number(raw);
    else data[key] = raw;
  }
  return data;
}

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

export async function scan(dir: string): Promise<ScannedSkill[]> {
  // Step 1: Root SKILL.md — standalone repo
  const rootSkillMd = join(dir, "SKILL.md");
  if (await Bun.file(rootSkillMd).exists()) {
    const skill = await processSkillFile(rootSkillMd);
    return [skill];
  }

  // Step 2: .agents/skills/*/SKILL.md
  const agentsGlob = new Bun.Glob(".agents/skills/*/SKILL.md");
  const agentsPaths: string[] = [];
  for await (const rel of agentsGlob.scan({
    cwd: dir,
    onlyFiles: true,
    dot: true,
  })) {
    agentsPaths.push(join(dir, rel));
  }

  // Step 2.5: skills/*/SKILL.md (antfu/skillpm convention) — priority path for npm packages
  const skillsGlob = new Bun.Glob("skills/*/SKILL.md");
  const skillsPaths: string[] = [];
  for await (const rel of skillsGlob.scan({
    cwd: dir,
    onlyFiles: true,
    dot: true,
  })) {
    skillsPaths.push(join(dir, rel));
  }

  // Step 3: Agent-specific paths (scanned in parallel — independent directories)
  const agentSpecificPatterns = [
    ".claude/skills/*/SKILL.md",
    ".cursor/skills/*/SKILL.md",
    ".codex/skills/*/SKILL.md",
    ".gemini/skills/*/SKILL.md",
    ".windsurf/skills/*/SKILL.md",
  ];
  const agentSpecificPaths = (
    await Promise.all(
      agentSpecificPatterns.map(async (pattern) => {
        const paths: string[] = [];
        for await (const rel of new Bun.Glob(pattern).scan({
          cwd: dir,
          onlyFiles: true,
          dot: true,
        })) {
          paths.push(join(dir, rel));
        }
        return paths;
      }),
    )
  ).flat();

  const combined = [...agentsPaths, ...skillsPaths, ...agentSpecificPaths];

  // Step 4: Deep scan fallback if nothing found
  const discoveredPaths = combined.length > 0 ? combined : await deepScan(dir);

  if (discoveredPaths.length === 0) return [];

  const skills = await Promise.all(discoveredPaths.map(processSkillFile));
  return deduplicate(skills);
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
