import { basename, join, resolve } from "node:path";
import { intro, outro } from "@clack/prompts";
import type { Output } from "@skilltap/core";
import { scan, validateSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { createOutput } from "../output";

export default defineCommand({
  meta: {
    name: "verify",
    description: "Validate a skill before sharing",
  },
  args: {
    path: {
      type: "positional",
      description: "Path to skill directory (default: .)",
      required: false,
    },
    all: {
      type: "boolean",
      description: "Verify all skills in the current project",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: false });
    const useJson = args.json as boolean;

    if (args.all) {
      await runAll(out, useJson);
      return;
    }

    const argPath = (args.path as string | undefined) ?? ".";
    let skillPath = resolve(argPath);

    // Bare-name fallback: try .agents/skills/<name> if arg has no path separator
    const isBare =
      !argPath.includes("/") && !argPath.includes("\\") && argPath !== ".";
    if (isBare) {
      const agentsPath = resolve(join(".", ".agents", "skills", argPath));
      if (await Bun.file(join(agentsPath, "SKILL.md")).exists()) {
        skillPath = agentsPath;
      }
    }

    const skillName = basename(skillPath);

    if (useJson) {
      await runJson(out, skillPath, skillName);
      return;
    }

    intro(`Verifying ${skillName}`);

    const p = out.progress("Running checks...");

    const result = await validateSkill(skillPath);

    if (!result.ok) {
      p.fail("Failed.");
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    p.succeed();

    const { issues, frontmatter, fileCount, totalBytes } = result.value;

    // SKILL.md found
    out.success("SKILL.md found");

    // Frontmatter
    if (frontmatter) {
      out.success("Frontmatter valid");
      out.raw(`   ${ansi.dim("name:")} ${frontmatter.name}\n`);
      out.raw(`   ${ansi.dim("description:")} ${frontmatter.description}\n`);
    }

    // Name match check
    const nameIssue = issues.find((i) =>
      i.message.includes("does not match directory name"),
    );
    if (!nameIssue) {
      out.success("Name matches directory");
    }

    // Security scan
    const scanWarnings = issues.filter(
      (i) => i.severity === "warning" && !i.message.includes("does not match"),
    );
    if (scanWarnings.length === 0) {
      out.success("Security scan: clean");
    } else {
      for (const w of scanWarnings) {
        out.raw(`  ${ansi.yellow("warning")}: ${w.message}\n`);
      }
    }

    // Size
    if (typeof totalBytes === "number" && typeof fileCount === "number") {
      const kb = (totalBytes / 1024).toFixed(1);
      const sizeOk = totalBytes <= 51200;
      if (sizeOk) {
        out.success(
          `Size: ${kb} KB (${fileCount} ${fileCount === 1 ? "file" : "files"})`,
        );
      } else {
        out.raw(`  ${ansi.yellow("warning")}: Size ${kb} KB exceeds 50 KB limit\n`);
      }
    }

    out.raw("\n");

    // Errors
    const errors = issues.filter((i) => i.severity === "error");
    if (errors.length > 0) {
      for (const e of errors) {
        out.raw(`  ${ansi.red("✗")} ${e.message}\n`);
      }
      out.raw("\n");
      outro(
        `✗ Fix ${errors.length} ${errors.length === 1 ? "issue" : "issues"} before sharing.`,
      );

      // Print tap.json snippet even on error so user knows what to aim for
      if (frontmatter) {
        printTapSnippet(out, frontmatter.name, frontmatter.description);
      }

      process.exit(1);
    }

    outro("✓ Skill is valid and ready to share.");

    if (frontmatter) {
      printTapSnippet(out, frontmatter.name, frontmatter.description);
    }
  },
});

async function runAll(out: Output, useJson: boolean): Promise<void> {
  const skills = await scan(process.cwd());

  if (skills.length === 0) {
    out.error("No skills found in current directory.");
    process.exit(1);
  }

  if (useJson) {
    const results = await Promise.all(
      skills.map(async (skill) => {
        const result = await validateSkill(skill.path);
        if (!result.ok) {
          return {
            name: skill.name,
            valid: false,
            error: result.error.message,
          };
        }
        const { valid, issues, frontmatter, fileCount, totalBytes } =
          result.value;
        return {
          name: skill.name,
          valid,
          issues,
          frontmatter: frontmatter ?? null,
          fileCount: fileCount ?? null,
          totalBytes: totalBytes ?? null,
        };
      }),
    );
    out.json(results);
    if (results.some((r) => !r.valid)) process.exit(1);
    return;
  }

  intro(
    `Verifying ${skills.length} ${skills.length === 1 ? "skill" : "skills"}`,
  );

  let passed = 0;
  let failed = 0;

  for (const skill of skills) {
    const p = out.progress(skill.name);
    const result = await validateSkill(skill.path);

    if (!result.ok) {
      p.fail(`${skill.name} — error: ${result.error.message}`);
      failed++;
      continue;
    }

    const { valid, issues } = result.value;
    const errors = issues.filter((i) => i.severity === "error");

    if (valid) {
      p.succeed(`${skill.name} — ✓ valid`);
      passed++;
    } else {
      p.fail(
        `${skill.name} — ✗ ${errors.length} ${errors.length === 1 ? "issue" : "issues"}`,
      );
      for (const e of errors) {
        out.raw(`    ${ansi.dim(e.message)}\n`);
      }
      failed++;
    }
  }

  out.raw("\n");

  if (failed === 0) {
    outro(`✓ All ${passed} ${passed === 1 ? "skill" : "skills"} valid.`);
  } else {
    outro(`${passed} passed, ${ansi.red(`${failed} failed`)}.`);
    process.exit(1);
  }
}

async function runJson(out: Output, skillPath: string, skillName: string): Promise<void> {
  const result = await validateSkill(skillPath);

  if (!result.ok) {
    out.json({ name: skillName, valid: false, error: result.error.message });
    process.exit(1);
  }

  const { valid, issues, frontmatter, fileCount, totalBytes } = result.value;

  out.json({
    name: skillName,
    valid,
    issues,
    frontmatter: frontmatter ?? null,
    fileCount: fileCount ?? null,
    totalBytes: totalBytes ?? null,
  });

  if (!valid) process.exit(1);
}

function printTapSnippet(out: Output, name: string, description: string): void {
  out.raw(`\n`);
  out.raw(
    `  ${ansi.dim("To make this discoverable via taps, add to your tap's tap.json:")}\n`,
  );
  out.raw(`  ${ansi.dim("{")} \n`);
  out.raw(`    ${ansi.dim(`"name": "${name}",`)}\n`);
  out.raw(`    ${ansi.dim(`"description": "${description}",`)}\n`);
  out.raw(`    ${ansi.dim(`"repo": "https://github.com/you/${name}",`)}\n`);
  out.raw(`    ${ansi.dim(`"tags": []`)}\n`);
  out.raw(`  ${ansi.dim("}")}\n`);
  out.raw(`\n`);
}
