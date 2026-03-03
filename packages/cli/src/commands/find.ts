import { autocomplete, isCancel, outro, spinner } from "@clack/prompts";
import type {
  Config,
  RegistrySearchResult,
  ScannedSkill,
  StaticWarning,
  TapEntry,
} from "@skilltap/core";
import {
  installSkill,
  loadConfig,
  loadTaps,
  resolveRegistries,
  searchRegistries,
  searchTaps,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  ansi,
  errorLine,
  formatInstallCount,
  successLine,
  table,
  termWidth,
  truncate,
} from "../ui/format";
import { confirmInstall, selectSkills, selectTap } from "../ui/prompts";
import { resolveScope } from "../ui/resolve";
import { printWarnings } from "../ui/scan";
import { formatTapTrust } from "../ui/trust";

type SearchEntry = {
  name: string;
  description: string;
  /** Tap name or "skills.sh" */
  source: string;
  /** Passed to installSkill (skill name or "owner/repo") */
  installRef: string;
  /** Pre-selected skill name for multi-skill repos */
  preSelectedSkill?: string;
  installs?: number;
  trustLabel?: string;
};

export default defineCommand({
  meta: {
    name: "find",
    description: "Search taps and skills.sh for skills",
  },
  args: {
    query: {
      type: "positional",
      description:
        "Search term (matched against name, description, tags)",
      required: false,
    },
    interactive: {
      type: "boolean",
      alias: "i",
      description: "Interactive search mode with type-ahead filtering",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    local: {
      type: "boolean",
      alias: "l",
      description: "Search local taps only (skip registries)",
      default: false,
    },
  },
  async run({ args }) {
    // Combine the first positional with any extra words from args._
    // so "skilltap find git hooks" works without quoting
    const rest = (args as Record<string, unknown>)._ as string[] | undefined;
    const parts = [args.query as string | undefined, ...(rest ?? [])].filter(Boolean);
    const query = parts.length > 0 ? parts.join(" ") : undefined;

    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const registries = args.local ? [] : resolveRegistries(config);

    // Collect results: taps + configured registries in parallel
    const [tapsResult, registrySkills] = await Promise.all([
      loadTaps(),
      query && query.length >= 2 && registries.length > 0
        ? searchRegistries(query, registries, 20)
        : Promise.resolve([] as RegistrySearchResult[]),
    ]);

    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    const tapEntries = tapsResult.value;
    const registryEntries: SearchEntry[] = registryToEntries(registrySkills);

    const tapSearchEntries: SearchEntry[] = tapEntries.map(
      ({ tapName, skill }) => ({
        name: skill.name,
        description: skill.description,
        source: tapName,
        installRef: skill.name,
        trustLabel: formatTapTrust(skill.trust),
      }),
    );

    if (tapEntries.length === 0 && !query) {
      process.stdout.write(
        `No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n`,
      );
      process.stdout.write(
        `Tip: search the skills.sh registry with 'skilltap find <query>'.\n`,
      );
      process.exit(0);
    }

    // Apply text filter to taps; registry results are already filtered by query
    const filtered = applyFilter(tapEntries, tapSearchEntries, registryEntries, query);

    if (filtered.length === 0) {
      process.stdout.write(
        query
          ? `No skills found matching '${query}'.\n`
          : "No skills found.\n",
      );
      process.exit(0);
    }

    if (args.json) {
      process.stdout.write(
        JSON.stringify(
          filtered.map((e) => ({
            name: e.name,
            description: e.description,
            source: e.source,
            installRef: e.installRef,
            ...(e.preSelectedSkill ? { skill: e.preSelectedSkill } : {}),
            ...(e.installs !== undefined ? { installs: e.installs } : {}),
          })),
          null,
          2,
        ),
      );
      process.stdout.write("\n");
      return;
    }

    if (args.interactive) {
      await runInteractive(filtered, config);
      return;
    }

    printTable(filtered);
  },
});

function registryToEntries(results: RegistrySearchResult[]): SearchEntry[] {
  return results.map((r) => {
    // Extract skill name from id when id = "{source}/{skill-name}"
    const skillName = r.id.startsWith(r.source + "/")
      ? r.id.slice(r.source.length + 1)
      : undefined;
    return {
      name: r.name,
      description: r.description,
      source: r.registryName,
      installRef: r.source,
      preSelectedSkill: skillName,
      installs: r.installs,
    };
  });
}

function applyFilter(
  tapEntries: TapEntry[],
  tapSearchEntries: SearchEntry[],
  registryEntries: SearchEntry[],
  query: string | undefined,
): SearchEntry[] {
  const filteredTaps = query
    ? searchTaps(tapEntries, query).map(({ tapName, skill }) => ({
        name: skill.name,
        description: skill.description,
        source: tapName,
        installRef: skill.name,
        trustLabel: formatTapTrust(skill.trust),
      }))
    : tapSearchEntries;

  // Sort registry results by install count (most popular first)
  const sortedRegistry = [...registryEntries].sort(
    (a, b) => (b.installs ?? 0) - (a.installs ?? 0),
  );

  return [...filteredTaps, ...sortedRegistry];
}

function printTable(entries: SearchEntry[]): void {
  const width = termWidth();
  const descWidth = Math.max(20, width - 66);
  const rows = entries.map((e) => [
    ansi.bold(e.name),
    truncate(e.description, descWidth),
    e.installs !== undefined ? ansi.dim(formatInstallCount(e.installs)) : (e.trustLabel ?? ""),
    ansi.dim(`[${e.source}]`),
  ]);

  process.stdout.write("\n");
  process.stdout.write(table(rows));
  process.stdout.write("\n\n");
}

// ---------------------------------------------------------------------------
// Interactive mode
// ---------------------------------------------------------------------------

async function runInteractive(
  entries: SearchEntry[],
  config: Config,
): Promise<void> {
  const result = await autocomplete({
    message: "Select a skill to install:",
    options: entries.map((entry, i) => ({
      value: i,
      label: entry.name,
      hint: `${entry.description || entry.source}  [${entry.source}]${entry.installs !== undefined ? `  ${formatInstallCount(entry.installs)}` : ""}`,
    })),
    placeholder: "Type to filter…",
  });

  if (isCancel(result)) process.exit(2);

  const chosen = entries[result as number];
  if (!chosen) process.exit(1);

  await installChosen(chosen, config);
}

async function installChosen(
  chosen: SearchEntry,
  config: Config,
): Promise<void> {
  const { scope, projectRoot } = await resolveScope({}, config);
  const also = config.defaults.also ?? [];

  const s = spinner();
  s.start(`Installing ${chosen.name}…`);

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
    s.start("Installing…");
    return true;
  };

  const selectSkillsCallback = async (
    skills_list: ScannedSkill[],
  ): Promise<string[]> => {
    // If we know which skill to install (from registry search), auto-select it
    if (chosen.preSelectedSkill) {
      const match = skills_list.find((sk) => sk.name === chosen.preSelectedSkill);
      if (match) return [match.name];
    }
    if (skills_list.length === 1) return skills_list.map((sk) => sk.name);
    s.stop();
    const selected = await selectSkills(skills_list);
    if (isCancel(selected)) process.exit(2);
    s.start("Installing…");
    return selected as string[];
  };

  const installResult = await installSkill(chosen.installRef, {
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
      s.start("Installing…");
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
