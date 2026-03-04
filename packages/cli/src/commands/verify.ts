import { intro, outro, spinner } from "@clack/prompts";
import { validateSkill, scan, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { resolve, basename, join } from "node:path";
import { agentError } from "../ui/agent-out";
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
    const useJson = args.json as boolean;
    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    if (args.all) {
      await runAll(useJson);
      return;
    }

    const argPath = (args.path as string | undefined) ?? ".";
    let skillPath = resolve(argPath);

    // Bare-name fallback: try .agents/skills/<name> if arg has no path separator
    const isBare = !argPath.includes("/") && !argPath.includes("\\") && argPath !== ".";
    if (isBare) {
      const agentsPath = resolve(join(".", ".agents", "skills", argPath));
      if (await Bun.file(join(agentsPath, "SKILL.md")).exists()) {
        skillPath = agentsPath;
      }
    }

    const skillName = basename(skillPath);

    if (useJson) {
      await runJson(skillPath, skillName);
      return;
    }

    if (agentMode) {
      const result = await validateSkill(skillPath);
      if (!result.ok) {
        agentError(result.error.message);
        process.exit(1);
      }
      const { valid, issues } = result.value;
      process.stdout.write(`${JSON.stringify({ name: skillName, valid, issues }, null, 2)}\n`);
      if (!valid) process.exit(1);
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

async function runAll(useJson: boolean): Promise<void> {
  const skills = await scan(process.cwd());

  if (skills.length === 0) {
    process.stderr.write("No skills found in current directory.\n");
    process.exit(1);
  }

  if (useJson) {
    const results = await Promise.all(
      skills.map(async (skill) => {
        const result = await validateSkill(skill.path);
        if (!result.ok) {
          return { name: skill.name, valid: false, error: result.error.message };
        }
        const { valid, issues, frontmatter, fileCount, totalBytes } = result.value;
        return { name: skill.name, valid, issues, frontmatter: frontmatter ?? null, fileCount: fileCount ?? null, totalBytes: totalBytes ?? null };
      }),
    );
    process.stdout.write(JSON.stringify(results, null, 2));
    process.stdout.write("\n");
    if (results.some((r) => !r.valid)) process.exit(1);
    return;
  }

  intro(`Verifying ${skills.length} ${skills.length === 1 ? "skill" : "skills"}`);

  let passed = 0;
  let failed = 0;

  for (const skill of skills) {
    const s = spinner();
    s.start(skill.name);
    const result = await validateSkill(skill.path);

    if (!result.ok) {
      s.stop(`${skill.name} — ${ansi.red("error")}: ${result.error.message}`, 1);
      failed++;
      continue;
    }

    const { valid, issues } = result.value;
    const errors = issues.filter((i) => i.severity === "error");

    if (valid) {
      s.stop(`${skill.name} — ${ansi.green("✓ valid")}`);
      passed++;
    } else {
      s.stop(`${skill.name} — ${ansi.red(`✗ ${errors.length} ${errors.length === 1 ? "issue" : "issues"}`)}`);
      for (const e of errors) {
        process.stdout.write(`    ${ansi.dim(e.message)}\n`);
      }
      failed++;
    }
  }

  process.stdout.write("\n");

  if (failed === 0) {
    outro(`✓ All ${passed} ${passed === 1 ? "skill" : "skills"} valid.`);
  } else {
    outro(`${passed} passed, ${ansi.red(`${failed} failed`)}.`);
    process.exit(1);
  }
}

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
