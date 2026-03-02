import { intro, outro, spinner } from "@clack/prompts";
import { validateSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { resolve, basename } from "node:path";
import { ansi, errorLine, successLine } from "../ui/format";

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
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const skillPath = resolve((args.path as string | undefined) ?? ".");
    const skillName = basename(skillPath);
    const useJson = args.json as boolean;

    if (useJson) {
      await runJson(skillPath, skillName);
      return;
    }

    intro(`Verifying ${skillName}`);

    const s = spinner();
    s.start("Running checks...");

    const result = await validateSkill(skillPath);

    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop();

    const { valid, issues, frontmatter, fileCount, totalBytes } = result.value;

    // SKILL.md found
    successLine("SKILL.md found");

    // Frontmatter
    if (frontmatter) {
      successLine("Frontmatter valid");
      process.stdout.write(`   ${ansi.dim("name:")} ${frontmatter.name}\n`);
      process.stdout.write(`   ${ansi.dim("description:")} ${frontmatter.description}\n`);
    }

    // Name match check
    const nameIssue = issues.find((i) => i.message.includes("does not match directory name"));
    if (!nameIssue) {
      successLine("Name matches directory");
    }

    // Security scan
    const scanWarnings = issues.filter(
      (i) => i.severity === "warning" && !i.message.includes("does not match"),
    );
    if (scanWarnings.length === 0) {
      successLine("Security scan: clean");
    } else {
      for (const w of scanWarnings) {
        process.stdout.write(`  ${ansi.yellow("warning")}: ${w.message}\n`);
      }
    }

    // Size
    if (typeof totalBytes === "number" && typeof fileCount === "number") {
      const kb = (totalBytes / 1024).toFixed(1);
      const sizeOk = totalBytes <= 51200;
      if (sizeOk) {
        successLine(`Size: ${kb} KB (${fileCount} ${fileCount === 1 ? "file" : "files"})`);
      } else {
        process.stdout.write(`  ${ansi.yellow("warning")}: Size ${kb} KB exceeds 50 KB limit\n`);
      }
    }

    process.stdout.write("\n");

    // Errors
    const errors = issues.filter((i) => i.severity === "error");
    if (errors.length > 0) {
      for (const e of errors) {
        process.stdout.write(`  ${ansi.red("✗")} ${e.message}\n`);
      }
      process.stdout.write("\n");
      outro(`✗ Fix ${errors.length} ${errors.length === 1 ? "issue" : "issues"} before sharing.`);

      // Print tap.json snippet even on error so user knows what to aim for
      if (frontmatter) {
        printTapSnippet(frontmatter.name, frontmatter.description);
      }

      process.exit(1);
    }

    outro("✓ Skill is valid and ready to share.");

    if (frontmatter) {
      printTapSnippet(frontmatter.name, frontmatter.description);
    }
  },
});

async function runJson(skillPath: string, skillName: string): Promise<void> {
  const result = await validateSkill(skillPath);

  if (!result.ok) {
    process.stdout.write(
      JSON.stringify({ name: skillName, valid: false, error: result.error.message }, null, 2),
    );
    process.stdout.write("\n");
    process.exit(1);
  }

  const { valid, issues, frontmatter, fileCount, totalBytes } = result.value;

  process.stdout.write(
    JSON.stringify(
      {
        name: skillName,
        valid,
        issues,
        frontmatter: frontmatter ?? null,
        fileCount: fileCount ?? null,
        totalBytes: totalBytes ?? null,
      },
      null,
      2,
    ),
  );
  process.stdout.write("\n");

  if (!valid) process.exit(1);
}

function printTapSnippet(name: string, description: string): void {
  process.stdout.write(`\n`);
  process.stdout.write(
    `  ${ansi.dim("To make this discoverable via taps, add to your tap's tap.json:")}\n`,
  );
  process.stdout.write(`  ${ansi.dim("{")} \n`);
  process.stdout.write(`    ${ansi.dim(`"name": "${name}",`)}\n`);
  process.stdout.write(`    ${ansi.dim(`"description": "${description}",`)}\n`);
  process.stdout.write(`    ${ansi.dim(`"repo": "https://github.com/you/${name}",`)}\n`);
  process.stdout.write(`    ${ansi.dim(`"tags": []`)}\n`);
  process.stdout.write(`  ${ansi.dim("}")}\n`);
  process.stdout.write(`\n`);
}
