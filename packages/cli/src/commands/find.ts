import { intro, isCancel, outro, spinner } from "@clack/prompts";
import type { ScannedSkill, StaticWarning, TapEntry } from "@skilltap/core";
import {
  installSkill,
  loadConfig,
  loadTaps,
  searchPackages,
  searchTaps,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  ansi,
  errorLine,
  successLine,
  table,
  termWidth,
  truncate,
} from "../ui/format";
import { confirmInstall, selectSkills, selectTap } from "../ui/prompts";
import { resolveScope } from "../ui/resolve";
import { printWarnings } from "../ui/scan";

export default defineCommand({
  meta: {
    name: "find",
    description: "Search taps for skills",
  },
  args: {
    query: {
      type: "positional",
      description:
        "Search term (fuzzy matched against name, description, tags)",
      required: false,
    },
    interactive: {
      type: "boolean",
      alias: "i",
      description: "Interactive fuzzy finder mode",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    npm: {
      type: "boolean",
      description: "Search npm registry instead of taps",
      default: false,
    },
  },
  async run({ args }) {
    const query = args.query as string | undefined;

    // npm registry search
    if (args.npm) {
      await runNpmSearch(query ?? "", args.json as boolean);
      return;
    }

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    const all = tapsResult.value;

    if (all.length === 0) {
      process.stdout.write(
        `No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n`,
      );
      process.exit(0);
    }

    const skills = query ? searchTaps(all, query) : all;

    if (skills.length === 0) {
      process.stdout.write(`No skills found matching '${query}'.\n`);
      process.exit(0);
    }

    if (args.json) {
      process.stdout.write(
        JSON.stringify(
          skills.map(({ tapName, skill }) => ({
            name: skill.name,
            description: skill.description,
            tap: tapName,
            tags: skill.tags,
            repo: skill.repo,
          })),
          null,
          2,
        ),
      );
      process.stdout.write("\n");
      return;
    }

    if (args.interactive) {
      await runInteractive(skills);
      return;
    }

    // Non-interactive table output
    const width = termWidth();
    const descWidth = Math.max(20, width - 40);
    const rows = skills.map(({ tapName, skill }) => [
      ansi.bold(skill.name),
      truncate(skill.description, descWidth),
      ansi.dim(`[${tapName}]`),
    ]);

    process.stdout.write("\n");
    process.stdout.write(table(rows));
    process.stdout.write("\n\n");
  },
});

async function runNpmSearch(query: string, json: boolean): Promise<void> {
  const result = await searchPackages(query, { keywords: ["agent-skill"] });
  if (!result.ok) {
    errorLine(result.error.message, result.error.hint);
    process.exit(1);
  }

  const packages = result.value;

  if (packages.length === 0) {
    process.stdout.write(
      query
        ? `No npm packages found matching '${query}'.\n`
        : "No npm packages found with the 'agent-skill' keyword.\n",
    );
    process.exit(0);
  }

  if (json) {
    process.stdout.write(
      JSON.stringify(
        packages.map((p) => ({
          name: p.name,
          version: p.version,
          description: p.description,
          source: "npm",
        })),
        null,
        2,
      ),
    );
    process.stdout.write("\n");
    return;
  }

  const width = termWidth();
  const descWidth = Math.max(20, width - 44);
  const rows = packages.map((p) => [
    ansi.bold(p.name),
    ansi.dim(p.version),
    truncate(p.description, descWidth),
    ansi.dim("[npm]"),
  ]);

  process.stdout.write("\n");
  process.stdout.write(table(rows));
  process.stdout.write("\n\n");
}

async function runInteractive(skills: TapEntry[]): Promise<void> {
  const { select } = await import("@clack/prompts");

  intro("skilltap find");

  const result = await select({
    message: "Select a skill to install:",
    options: skills.map((entry, i) => ({
      value: i,
      label: `${entry.skill.name}`,
      hint: `[${entry.tapName}] ${entry.skill.description}`,
    })),
  });

  if (isCancel(result)) {
    process.exit(2);
  }

  // biome-ignore lint/style/noNonNullAssertion: result is a valid index from the select options
  const chosen = skills[result as number]!;

  // Load config for defaults
  const configResult = await loadConfig();
  if (!configResult.ok) {
    errorLine(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  const { scope, projectRoot } = await resolveScope({}, config);

  const also = config.defaults.also ?? [];

  const s = spinner();
  s.start(`Installing ${chosen.skill.name}...`);

  const warningsCallback = async (
    warnings: StaticWarning[],
    skillName: string,
  ): Promise<boolean> => {
    s.stop();
    printWarnings(warnings, skillName);
    if (config.security.on_warn === "fail") {
      errorLine(`Security warnings found in ${skillName} — aborting`);
      process.exit(1);
    }
    const proceed = await confirmInstall(skillName);
    if (isCancel(proceed) || proceed === false) process.exit(2);
    s.start("Installing...");
    return true;
  };

  const selectSkillsCallback = async (
    skills_list: ScannedSkill[],
  ): Promise<string[]> => {
    if (skills_list.length === 1) return skills_list.map((sk) => sk.name);
    s.stop();
    const selected = await selectSkills(skills_list);
    if (isCancel(selected)) process.exit(2);
    s.start("Installing...");
    return selected as string[];
  };

  const installResult = await installSkill(chosen.skill.name, {
    scope,
    projectRoot,
    also,
    skipScan: false,
    onWarnings: warningsCallback,
    onSelectSkills: selectSkillsCallback,
    onSelectTap: async (matches: TapEntry[]) => {
      s.stop();
      const selected = await selectTap(matches);
      if (isCancel(selected)) process.exit(2);
      s.start("Installing...");
      return selected as TapEntry;
    },
  });

  if (!installResult.ok) {
    s.stop("Failed.", 1);
    errorLine(installResult.error.message, installResult.error.hint);
    process.exit(1);
  }

  s.stop("Done.");
  for (const record of installResult.value.records) {
    successLine(`Installed ${record.name}`);
  }
  outro("Complete!");
}
