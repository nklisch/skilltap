import { cancel, intro, isCancel, outro } from "@clack/prompts";
import { footerSelect as select, footerText as text } from "../ui/footer";
import { basicTemplate, multiTemplate, npmTemplate, TEMPLATE_NAMES } from "@skilltap/core";
import { defineCommand } from "citty";
import { dirname, join, resolve } from "node:path";
import { mkdir } from "node:fs/promises";
import { ansi, errorLine, successLine } from "../ui/format";

const NAME_REGEX = /^[a-z0-9]+(-[a-z0-9]+)*$/;

function validateName(name: string | undefined): string | undefined {
  if (!name) return "Skill name is required";
  if (name.length > 64) return "Skill name must be 64 characters or fewer";
  if (!NAME_REGEX.test(name))
    return "Skill name must be lowercase alphanumeric with hyphens (e.g. my-skill)";
}

async function getGitAuthor(): Promise<string> {
  try {
    const proc = await Bun.$`git config user.name`.quiet().nothrow();
    const name = proc.stdout.toString().trim();
    if (name) return name;
  } catch {}
  return "";
}

async function writeFiles(
  dir: string,
  files: Record<string, string>,
): Promise<void> {
  for (const [relPath, content] of Object.entries(files)) {
    const filePath = join(dir, relPath);
    await mkdir(dirname(filePath), { recursive: true });
    await Bun.write(filePath, content);
  }
}

function printNextSteps(
  name: string,
  dir: string,
  template: string,
  fileList: string[],
): void {
  process.stdout.write(`\n`);
  successLine(`Created ${dir}/`);
  for (const f of fileList) {
    process.stdout.write(`    ${ansi.dim("├──")} ${f}\n`);
  }
  process.stdout.write(`\n`);
  process.stdout.write(`  ${ansi.bold("Next steps:")}\n`);
  process.stdout.write(`    cd ${dir}\n`);
  process.stdout.write(`    ${ansi.dim("# Edit SKILL.md with your skill instructions")}\n`);
  if (template === "npm") {
    process.stdout.write(
      `    ${ansi.dim("# Edit package.json — set \"name\" to your npm scope (e.g. @yourname/${name})")}\n`,
    );
    process.stdout.write(`    ${ansi.dim("# Set repository.url in package.json")}\n`);
  }
  process.stdout.write(`    skilltap link . --also claude-code   ${ansi.dim("# Test locally")}\n`);
  process.stdout.write(`    skilltap verify                        ${ansi.dim("# Validate before sharing")}\n`);
  process.stdout.write(`    git init && git add -A && git commit -m "Initial skill"\n`);
  if (template === "npm") {
    process.stdout.write(`    ${ansi.dim("# Push, then create a GitHub release to trigger publish")}\n`);
  } else {
    process.stdout.write(
      `    git remote add origin <your-git-url> && git push -u origin main\n`,
    );
  }
  process.stdout.write(`\n`);
}

export default defineCommand({
  meta: {
    name: "create",
    description: "Scaffold a new skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name (kebab-case)",
      required: false,
    },
    template: {
      type: "string",
      alias: "t",
      description: "Template to use: basic, npm, multi",
      default: "",
    },
    dir: {
      type: "string",
      description: "Output directory (default: ./{name})",
      default: "",
    },
  },
  async run({ args }) {
    const author = await getGitAuthor();

    // Non-interactive mode: name and template both provided
    const isNonInteractive =
      !!(args.name as string | undefined) && !!(args.template as string);

    if (isNonInteractive) {
      await runNonInteractive({
        name: args.name as string,
        template: (args.template as string) || "basic",
        dir: (args.dir as string) || "",
        author,
      });
      return;
    }

    // Interactive mode
    intro("Create a new skill");

    // Name
    let name = args.name as string | undefined;
    if (!name) {
      const result = await text({
        message: "Skill name?",
        placeholder: "my-skill",
        validate: validateName,
      });
      if (isCancel(result)) {
        cancel("Cancelled.");
        process.exit(2);
      }
      name = result as string;
    } else {
      const err = validateName(name);
      if (err) {
        errorLine(err, "Skill name must match /^[a-z0-9]+(-[a-z0-9]+)*$/");
        process.exit(1);
      }
    }

    // Description
    const descResult = await text({
      message: "Description?",
      placeholder: "A brief description of what this skill does",
      validate(v) {
        if (!v) return "Description is required";
      },
    });
    if (isCancel(descResult)) {
      cancel("Cancelled.");
      process.exit(2);
    }
    const description = descResult as string;

    // Template
    let template = (args.template as string) || "";
    if (!template) {
      const templateResult = await select({
        message: "Template?",
        options: [
          {
            value: "basic",
            label: "Basic — standalone git repo",
            hint: "recommended",
          },
          { value: "npm", label: "npm — publishable to npm with provenance" },
          { value: "multi", label: "Multi — multiple skills in one repo" },
        ],
      });
      if (isCancel(templateResult)) {
        cancel("Cancelled.");
        process.exit(2);
      }
      template = templateResult as string;
    }

    // Skill names for multi template
    let skillNames: string[] = [];
    if (template === "multi") {
      const skillNamesResult = await text({
        message: "Skill names? (comma-separated)",
        placeholder: "skill-a, skill-b",
        validate(v) {
          if (!v) return "At least one skill name is required";
          const names = v.split(",").map((s) => s.trim());
          for (const n of names) {
            const err = validateName(n);
            if (err) return `Invalid name "${n}": ${err}`;
          }
        },
      });
      if (isCancel(skillNamesResult)) {
        cancel("Cancelled.");
        process.exit(2);
      }
      skillNames = (skillNamesResult as string).split(",").map((s) => s.trim());
    }

    // License
    const licenseResult = await select({
      message: "License?",
      options: [
        { value: "MIT", label: "MIT" },
        { value: "Apache-2.0", label: "Apache-2.0" },
        { value: "None", label: "None" },
      ],
    });
    if (isCancel(licenseResult)) {
      cancel("Cancelled.");
      process.exit(2);
    }
    const license = licenseResult as string;

    const outDir = (args.dir as string) || name;
    await createSkill({ name, description, license, author, template, skillNames, outDir });

    outro("Done!");
  },
});

async function runNonInteractive(opts: {
  name: string;
  template: string;
  dir: string;
  author: string;
}): Promise<void> {
  const { name, template, dir: dirFlag, author } = opts;

  const nameErr = validateName(name);
  if (nameErr) {
    errorLine(nameErr, "Skill name must match /^[a-z0-9]+(-[a-z0-9]+)*$/");
    process.exit(1);
  }

  if (!(TEMPLATE_NAMES as readonly string[]).includes(template)) {
    errorLine(`Unknown template '${template}'`, `Use: ${TEMPLATE_NAMES.join(", ")}`);
    process.exit(1);
  }

  const outDir = dirFlag || name;
  const description = `${name} skill`;
  const license = "MIT";
  const skillNames = template === "multi" ? [`${name}-a`, `${name}-b`] : [];

  await createSkill({ name, description, license, author, template, skillNames, outDir });
}

async function createSkill(opts: {
  name: string;
  description: string;
  license: string;
  author: string;
  template: string;
  skillNames: string[];
  outDir: string;
}): Promise<void> {
  const { name, description, license, author, template, skillNames, outDir } = opts;
  const resolvedDir = resolve(outDir);

  // Check directory doesn't already exist
  try {
    await Bun.file(resolvedDir).exists(); // will be false for dirs
    // Use lstat to check for actual directory
    const { lstat } = await import("node:fs/promises");
    try {
      await lstat(resolvedDir);
      errorLine(`Directory '${outDir}/' already exists.`, "Use --dir to specify a different location.");
      process.exit(1);
    } catch {
      // doesn't exist — good
    }
  } catch {
    // ignore
  }

  let files: Record<string, string>;
  switch (template) {
    case "npm":
      files = npmTemplate({ name, description, license, author });
      break;
    case "multi":
      files = multiTemplate({ description, license, author, skillNames });
      break;
    default:
      files = basicTemplate({ name, description, license, author });
  }

  await writeFiles(resolvedDir, files);

  const fileList = Object.keys(files).sort();
  printNextSteps(name, outDir, template, fileList);
}
