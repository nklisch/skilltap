import { isCancel, outro, S_RADIO_ACTIVE, S_RADIO_INACTIVE, spinner } from "@clack/prompts";
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
import pc from "picocolors";
import {
  ansi,
  errorLine,
  formatInstallCount,
  highlightMatches,
  successLine,
  table,
  termWidth,
  truncate,
} from "../ui/format";
import { confirmInstall, selectSkills, selectTap } from "../ui/prompts";
import { resolveScope } from "../ui/resolve";
import { printWarnings } from "../ui/scan";
import { searchPrompt } from "../ui/search-prompt";
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

    const isTTY = process.stdout.isTTY === true;
    const wantInteractive = args.interactive || (!query && !args.json && isTTY);

    if (wantInteractive) {
      await runInteractiveSearch(query, args.local, config);
      return;
    }

    // Non-interactive path
    const { filtered, tapEntries } = await search(query ?? "", args.local, config);

    if (filtered.length === 0 && !query) {
      if (tapEntries.length === 0) {
        process.stdout.write(
          "No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n",
        );
        process.stdout.write(
          "Tip: search the skills.sh registry with 'skilltap find <query>'.\n",
        );
      } else {
        process.stdout.write("No skills found.\n");
      }
      process.exit(0);
    }

    if (filtered.length === 0) {
      process.stdout.write(`No skills found matching '${query}'.\n`);
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

    printTable(filtered);
  },
});

// ---------------------------------------------------------------------------
// Search helpers
// ---------------------------------------------------------------------------

async function search(
  query: string,
  local: boolean,
  config: Config,
): Promise<{ filtered: SearchEntry[]; tapEntries: TapEntry[] }> {
  const registries = local ? [] : resolveRegistries(config);

  const [tapsResult, registrySkills] = await Promise.all([
    loadTaps(),
    query.length >= 2 && registries.length > 0
      ? searchRegistries(query, registries, 20)
      : Promise.resolve([] as RegistrySearchResult[]),
  ]);

  if (!tapsResult.ok) {
    errorLine(tapsResult.error.message, tapsResult.error.hint);
    process.exit(1);
  }

  const tapEntries = tapsResult.value;
  const registryEntries = registryToEntries(registrySkills);
  const tapSearchEntries: SearchEntry[] = tapEntries.map(
    ({ tapName, skill }) => ({
      name: skill.name,
      description: skill.description,
      source: tapName,
      installRef: skill.name,
      trustLabel: formatTapTrust(skill.trust),
    }),
  );

  const filtered = applyFilter(tapEntries, tapSearchEntries, registryEntries, query);
  return { filtered, tapEntries };
}

function registryToEntries(results: RegistrySearchResult[]): SearchEntry[] {
  return results.map((r) => {
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

  const sortedRegistry = [...registryEntries].sort(
    (a, b) => (b.installs ?? 0) - (a.installs ?? 0),
  );

  return [...filteredTaps, ...sortedRegistry];
}

// ---------------------------------------------------------------------------
// Table output
// ---------------------------------------------------------------------------

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

async function runInteractiveSearch(
  initialQuery: string | undefined,
  local: boolean,
  config: Config,
): Promise<void> {
  const width = termWidth();
  const maxLabelWidth = Math.max(40, width - 10);

  const result = await searchPrompt<SearchEntry>({
    message: "Search for skills:",
    placeholder: "e.g. git, testing, docker…",
    initialQuery,
    debounce: 250,
    source: async (query) => {
      if (!query || query.trim().length < 2) {
        // Short queries: tap skills only (no registry API call)
        const tapsResult = await loadTaps();
        if (!tapsResult.ok) return [];
        return tapsResult.value.map(({ tapName, skill }) => ({
          name: skill.name,
          description: skill.description,
          source: tapName,
          installRef: skill.name,
          trustLabel: formatTapTrust(skill.trust),
        }));
      }
      const { filtered } = await search(query, local, config);
      return filtered;
    },
    selector: (entry) => `${entry.name} ${entry.description}`,
    renderItem: (entry, active, positions) => {
      const installs =
        entry.installs !== undefined
          ? formatInstallCount(entry.installs)
          : "";
      const source = `[${entry.source}]`;
      const meta = installs ? `${source}  ${installs}` : source;

      // Layout: name  description  [source]  installs
      const rawName = truncate(entry.name, 30);
      const name = positions
        ? highlightMatches(rawName, positions)
        : rawName;

      // Fill description into remaining space between name and meta
      const fixedCols = rawName.length + 2 + meta.length + 2; // name + gaps + meta
      const descSpace = Math.max(0, maxLabelWidth - fixedCols);
      const desc = entry.description
        ? truncate(entry.description, descSpace)
        : "";
      const pad = Math.max(1, maxLabelWidth - rawName.length - desc.length - meta.length - 2);

      if (active) {
        return `${pc.green(S_RADIO_ACTIVE)} ${name}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(meta)}`;
      }
      return `${pc.dim(S_RADIO_INACTIVE)} ${pc.dim(rawName)}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(meta)}`;
    },
  });

  if (isCancel(result)) process.exit(2);
  await installChosen(result as SearchEntry, config);
}

// ---------------------------------------------------------------------------
// Install from picker
// ---------------------------------------------------------------------------

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
