import {
  loadConfigIfExists,
  type Output,
  type TryReport,
  type TryType,
  tryPreview,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { setupOutput } from "../ui/setup";

const VALID_TRY_TYPES = [
  "skill",
  "plugin",
  "mcp",
] as const satisfies readonly TryType[];

export default defineCommand({
  meta: {
    name: "try",
    description: "Preview a skill, plugin, or MCP server without installing",
  },
  args: {
    type: {
      type: "positional",
      description: "skill | plugin | mcp",
      required: true,
    },
    source: {
      type: "positional",
      description:
        "Source URL, owner/repo shorthand, npm: prefix, or local path",
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
    const out = setupOutput(args);

    const typeArg = args.type as string;
    if (!VALID_TRY_TYPES.includes(typeArg as TryType)) {
      out.error(
        `Invalid try type '${typeArg}'.`,
        `Use 'skill', 'plugin', or 'mcp'. Example: skilltap try skill owner/repo`,
      );
      process.exit(1);
    }
    const type = typeArg as TryType;

    const configResult = await loadConfigIfExists();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }

    const result = await tryPreview(args.source as string, {
      type,
      gitHost: configResult.value.default_git_host,
      skipScan: args["skip-scan"] as boolean,
    });
    if (!result.ok) {
      out.error(result.error.message);
      if (result.error.hint) {
        out.warn(result.error.hint);
      }
      process.exit(1);
    }

    if (args.json as boolean) {
      out.json(reportToJson(result.value));
      return;
    }

    renderTry(out, result.value);
  },
});

function reportToJson(report: TryReport): unknown {
  return {
    source: report.source,
    type: report.type,
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

function renderTry(out: Output, report: TryReport): void {
  out.info(
    `\n${ansi.bold("skilltap try")} ${ansi.dim("—")} ${report.source}\n`,
  );
  out.info(
    `${ansi.dim("Resolved:")} ${report.resolved.url}${report.resolved.ref ? ansi.dim(`@${report.resolved.ref}`) : ""}`,
  );
  if (report.sha) {
    out.info(`${ansi.dim("SHA:")} ${report.sha}`);
  }
  out.info("");

  if (report.plugin) {
    out.info(
      `${ansi.bold("Plugin:")} ${report.plugin.name} ${ansi.dim(`(${report.plugin.format})`)}`,
    );
    const skillCount = report.plugin.components.filter(
      (c) => c.type === "skill",
    ).length;
    const mcpCount = report.plugin.components.filter(
      (c) => c.type === "mcp",
    ).length;
    const agentCount = report.plugin.components.filter(
      (c) => c.type === "agent",
    ).length;
    out.info(
      `  ${skillCount} skill${skillCount === 1 ? "" : "s"}, ${mcpCount} MCP server${mcpCount === 1 ? "" : "s"}, ${agentCount} agent${agentCount === 1 ? "" : "s"}\n`,
    );
  }

  if (report.skills.length > 0) {
    out.info(`${ansi.bold("Skills")} ${ansi.dim(`(${report.skills.length})`)}`);
    for (const skill of report.skills) {
      const desc = skill.description ? ansi.dim(` — ${skill.description}`) : "";
      out.info(`  ${skill.name}${desc}`);
    }
    out.info("");
  } else if (!report.plugin) {
    out.info(`${ansi.dim("Skills:")} ${ansi.dim("(none found)")}\n`);
  }

  if (!report.scanned) {
    out.info(`${ansi.dim("Scan:")} skipped\n`);
  } else if (report.warnings.length === 0) {
    out.info(`${ansi.green("✓")} No security warnings.\n`);
  } else {
    out.info(
      `${ansi.yellow("⚠")} ${report.warnings.length} security warning${report.warnings.length === 1 ? "" : "s"}:`,
    );
    for (const w of report.warnings) {
      const lineLabel =
        typeof w.line === "number" && w.line > 0 ? `:${w.line}` : "";
      out.info(`  ${ansi.yellow(w.category)} ${w.file}${lineLabel}`);
    }
    out.info("");
  }

  out.info(`${ansi.dim("This was a preview. Nothing was installed.")}`);
  out.info(
    `${ansi.dim(`To install: skilltap install ${report.type} `)}${report.source}`,
  );
}
