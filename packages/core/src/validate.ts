import { basename, join } from "node:path";
import { parseSkillFrontmatter } from "./frontmatter";
import { SkillFrontmatterSchema } from "./schemas";
import type { SkillFrontmatter } from "./schemas";
import { scanStatic } from "./security";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export interface ValidationIssue {
  severity: "error" | "warning";
  message: string;
}

export interface ValidationResult {
  valid: boolean;
  issues: ValidationIssue[];
  frontmatter?: SkillFrontmatter;
  fileCount?: number;
  totalBytes?: number;
}

/**
 * Validate a skill directory. Checks:
 * 1. SKILL.md exists
 * 2. Frontmatter parses and validates against SkillFrontmatterSchema
 * 3. Name matches directory name
 * 4. Static security scan (warnings → validation warnings)
 * 5. Size within 50KB limit
 */
export async function validateSkill(
  dir: string,
): Promise<Result<ValidationResult, UserError>> {
  const issues: ValidationIssue[] = [];

  // 1. SKILL.md exists
  const skillMdPath = join(dir, "SKILL.md");
  if (!(await Bun.file(skillMdPath).exists())) {
    return err(
      new UserError(
        `No SKILL.md found in '${dir}'`,
        "Create one with: skilltap create",
      ),
    );
  }

  // 2. Parse and validate frontmatter
  let content: string;
  try {
    content = await Bun.file(skillMdPath).text();
  } catch (e) {
    return err(
      new UserError(`Could not read SKILL.md: ${e instanceof Error ? e.message : String(e)}`),
    );
  }

  const rawFm = parseSkillFrontmatter(content);
  if (!rawFm) {
    issues.push({ severity: "error", message: "No YAML frontmatter found in SKILL.md" });
    return ok({ valid: false, issues });
  }

  const parsed = SkillFrontmatterSchema.safeParse(rawFm);
  if (!parsed.success) {
    for (const issue of parsed.error.issues) {
      issues.push({ severity: "error", message: issue.message });
    }
    return ok({ valid: false, issues });
  }

  const frontmatter = parsed.data;

  // 3. Name matches directory name
  const dirName = basename(dir);
  if (frontmatter.name !== dirName) {
    issues.push({
      severity: "error",
      message: `Skill name "${frontmatter.name}" does not match directory name "${dirName}"`,
    });
  }

  // 4. Static security scan
  const scanResult = await scanStatic(dir);
  if (scanResult.ok) {
    for (const w of scanResult.value) {
      issues.push({
        severity: "warning",
        message: `${w.category} in ${w.file}: ${w.raw}`,
      });
    }
  }

  // 5. Collect file count and total size
  let fileCount = 0;
  let totalBytes = 0;
  try {
    const glob = new Bun.Glob("**/*");
    for await (const relPath of glob.scan({ cwd: dir, onlyFiles: true, dot: true })) {
      if (relPath.startsWith(".git/") || relPath.startsWith(".svn/") || relPath.startsWith(".hg/")) continue;
      fileCount++;
      totalBytes += Bun.file(join(dir, relPath)).size;
    }
  } catch {
    // non-fatal
  }

  const hasErrors = issues.some((i) => i.severity === "error");
  return ok({ valid: !hasErrors, issues, frontmatter, fileCount, totalBytes });
}
