import { tryPreview, type TryReport } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine } from "../ui/format";

export default defineCommand({
  meta: {
    name: "try",
    description: "Preview a skill or plugin without installing",
  },
  args: {
    source: {
      type: "positional",
      description: "Source URL, owner/repo shorthand, npm: prefix, or local path",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip the static security scan",
      default: false,
    },
  },
  async run({ args }) {
    const result = await tryPreview(args.source as string, {
      skipScan: args["skip-scan"] as boolean,
    });
    if (!result.ok) {
      errorLine(result.error.message);
      if (result.error.hint) {
        process.stderr.write(`${ansi.dim("hint:")} ${result.error.hint}\n`);
      }
      process.exit(1);
    }

    if (args.json as boolean) {
      process.stdout.write(`${JSON.stringify(reportToJson(result.value), null, 2)}\n`);
      return;
    }

    renderTry(result.value);
  },
});

function reportToJson(report: TryReport): unknown {
  return {
    source: report.source,
    resolved: report.resolved,
    sha: report.sha,
    plugin: report.plugin
      ? {
          name: report.plugin.name,
          format: report.plugin.format,
          components: report.plugin.components.length,
        }
      : null,
    skills: report.skills.map((s) => ({
      name: s.name,
      description: s.description,
    })),
    warnings: report.warnings.map((w) => ({
      category: w.category,
      file: w.file,
      line: w.line,
    })),
    scanned: report.scanned,
  };
}

function renderTry(report: TryReport): void {
  process.stdout.write(`\n${ansi.bold("skilltap try")} ${ansi.dim("—")} ${report.source}\n\n`);
  process.stdout.write(
    `${ansi.dim("Resolved:")} ${report.resolved.url}${report.resolved.ref ? ansi.dim(`@${report.resolved.ref}`) : ""}\n`,
  );
  if (report.sha) {
    process.stdout.write(`${ansi.dim("SHA:")} ${report.sha}\n`);
  }
  process.stdout.write("\n");

  if (report.plugin) {
    process.stdout.write(
      `${ansi.bold("Plugin:")} ${report.plugin.name} ${ansi.dim(`(${report.plugin.format})`)}\n`,
    );
    const skillCount = report.plugin.components.filter((c) => c.type === "skill").length;
    const mcpCount = report.plugin.components.filter((c) => c.type === "mcp").length;
    const agentCount = report.plugin.components.filter((c) => c.type === "agent").length;
    process.stdout.write(
      `  ${skillCount} skill${skillCount === 1 ? "" : "s"}, ${mcpCount} MCP server${mcpCount === 1 ? "" : "s"}, ${agentCount} agent${agentCount === 1 ? "" : "s"}\n\n`,
    );
  }

  if (report.skills.length > 0) {
    process.stdout.write(
      `${ansi.bold("Skills")} ${ansi.dim(`(${report.skills.length})`)}\n`,
    );
    for (const skill of report.skills) {
      const desc = skill.description ? ansi.dim(` — ${skill.description}`) : "";
      process.stdout.write(`  ${skill.name}${desc}\n`);
    }
    process.stdout.write("\n");
  } else if (!report.plugin) {
    process.stdout.write(`${ansi.dim("Skills:")} ${ansi.dim("(none found)")}\n\n`);
  }

  if (!report.scanned) {
    process.stdout.write(`${ansi.dim("Scan:")} skipped\n\n`);
  } else if (report.warnings.length === 0) {
    process.stdout.write(`${ansi.green("✓")} No security warnings.\n\n`);
  } else {
    process.stdout.write(
      `${ansi.yellow("⚠")} ${report.warnings.length} security warning${report.warnings.length === 1 ? "" : "s"}:\n`,
    );
    for (const w of report.warnings) {
      const lineLabel = typeof w.line === "number" && w.line > 0 ? `:${w.line}` : "";
      process.stdout.write(`  ${ansi.yellow(w.category)} ${w.file}${lineLabel}\n`);
    }
    process.stdout.write("\n");
  }

  process.stdout.write(`${ansi.dim("This was a preview. Nothing was installed.")}\n`);
  process.stdout.write(
    `${ansi.dim("To install: skilltap install ")}${report.source}\n`,
  );
}
