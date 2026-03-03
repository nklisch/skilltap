import { isCancel, outro, select, spinner, text } from "@clack/prompts";
import { $ } from "bun";
import type { Config, NpmSearchResult, ScannedSkill, StaticWarning, TapEntry } from "@skilltap/core";
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
import { formatTapTrust } from "../ui/trust";

type SearchEntry = {
  name: string;
  description: string;
  /** Tap name or "npm" */
  source: string;
  /** Passed to installSkill (skill name or "npm:package") */
  installRef: string;
  version?: string;
  trustLabel?: string;
};

export default defineCommand({
  meta: {
    name: "find",
    description: "Search taps for skills",
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
      description: "Interactive search mode (fzf if available, otherwise guided)",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    npm: {
      type: "boolean",
      description: "Search npm registry only (auto-included when registry.allow_npm = true)",
      default: false,
    },
  },
  async run({ args }) {
    const query = args.query as string | undefined;

    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    // --npm flag: search npm only
    if (args.npm) {
      if (!config.registry.allow_npm) {
        errorLine(
          "npm registry search is disabled by config (registry.allow_npm = false).",
          "To allow npm search, set allow_npm = true via 'skilltap config'.",
        );
        process.exit(1);
      }
      const entries = await fetchNpmEntries(query ?? "");
      const filtered = query ? filterEntries(entries, query) : entries;
      outputResults(filtered, query, args.json as boolean, false);
      return;
    }

    // Collect results: taps always, npm when allow_npm = true
    const [tapsResult, npmResult] = await Promise.all([
      loadTaps(),
      config.registry.allow_npm
        ? searchPackages(query ?? "", { keywords: ["agent-skill"] })
        : Promise.resolve(null),
    ]);

    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    const tapEntries = tapsResult.value;
    const npmEntries: SearchEntry[] =
      npmResult && npmResult.ok ? npmToEntries(npmResult.value) : [];

    const tapSearchEntries: SearchEntry[] = tapEntries.map(({ tapName, skill }) => ({
      name: skill.name,
      description: skill.description,
      source: tapName,
      installRef: skill.name,
      trustLabel: formatTapTrust(skill.trust),
    }));

    const hasNpm = npmEntries.length > 0;

    if (tapEntries.length === 0 && !hasNpm) {
      process.stdout.write(
        `No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n`,
      );
      if (!config.registry.allow_npm) {
        process.stdout.write(
          `Tip: enable npm registry search via 'skilltap config'.\n`,
        );
      }
      process.exit(0);
    }

    // Apply text filter if query given
    const filtered = applyFilter(tapEntries, tapSearchEntries, npmEntries, query);

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
            ...(e.version ? { version: e.version } : {}),
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

function npmToEntries(packages: NpmSearchResult[]): SearchEntry[] {
  return packages.map((p) => ({
    name: p.name,
    description: p.description,
    source: "npm",
    installRef: `npm:${p.name}`,
    version: p.version,
    trustLabel: ansi.dim("● publisher"),
  }));
}

async function fetchNpmEntries(query: string): Promise<SearchEntry[]> {
  const result = await searchPackages(query, { keywords: ["agent-skill"] });
  if (!result.ok) {
    errorLine(result.error.message, result.error.hint);
    process.exit(1);
  }
  return npmToEntries(result.value);
}

function filterEntries(entries: SearchEntry[], query: string): SearchEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return entries;
  return entries.filter(
    (e) =>
      e.name.toLowerCase().includes(q) ||
      e.description.toLowerCase().includes(q),
  );
}

function applyFilter(
  tapEntries: TapEntry[],
  tapSearchEntries: SearchEntry[],
  npmEntries: SearchEntry[],
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

  const filteredNpm = query ? filterEntries(npmEntries, query) : npmEntries;

  return [...filteredTaps, ...filteredNpm];
}

function printTable(entries: SearchEntry[]): void {
  const width = termWidth();
  const descWidth = Math.max(20, width - 60);
  const rows = entries.map((e) => [
    ansi.bold(e.name),
    truncate(e.description, descWidth),
    e.trustLabel ?? "",
    e.version ? ansi.dim(e.version) : "",
    ansi.dim(`[${e.source}]`),
  ]);

  process.stdout.write("\n");
  process.stdout.write(table(rows));
  process.stdout.write("\n\n");
}

function outputResults(
  entries: SearchEntry[],
  query: string | undefined,
  json: boolean,
  _interactive: boolean,
): void {
  if (entries.length === 0) {
    process.stdout.write(
      query
        ? `No packages found matching '${query}'.\n`
        : "No packages found with the 'agent-skill' keyword.\n",
    );
    process.exit(0);
  }

  if (json) {
    process.stdout.write(JSON.stringify(entries, null, 2));
    process.stdout.write("\n");
    return;
  }

  printTable(entries);
}

// ---------------------------------------------------------------------------
// Interactive mode
// ---------------------------------------------------------------------------

async function runInteractive(
  entries: SearchEntry[],
  config: Config,
): Promise<void> {
  // Try fzf first
  const fzfPath = await $`which fzf`.quiet().text().catch(() => "");
  if (fzfPath.trim()) {
    const chosen = await runFzf(entries, fzfPath.trim());
    if (chosen) {
      await installChosen(chosen, config);
    }
    return;
  }

  // Fallback: text search → clack select
  await runGuidedSearch(entries, config);
}

async function runFzf(
  entries: SearchEntry[],
  fzfBin: string,
): Promise<SearchEntry | null> {
  // Format: "{index}\t{name}\t{description}\t[{source}]"
  const lines = entries.map(
    (e, i) => `${i}\t${e.name}\t${e.description}${e.version ? ` ${e.version}` : ""}\t[${e.source}]`,
  );
  const input = lines.join("\n");

  const proc = Bun.spawn(
    [
      fzfBin,
      "--delimiter",
      "\t",
      "--with-nth",
      "2..",
      "--ansi",
      "--height",
      "~50%",
      "--min-height",
      "10",
      "--reverse",
      "--prompt",
      "Search skills: ",
      "--info",
      "inline",
    ],
    {
      stdin: "pipe",
      stdout: "pipe",
      stderr: "inherit",
    },
  );

  proc.stdin.write(input);
  proc.stdin.end();

  const output = await new Response(proc.stdout).text();
  const exitCode = await proc.exited;

  if (exitCode !== 0 || !output.trim()) return null;

  const idx = parseInt(output.split("\t")[0] ?? "");
  return Number.isNaN(idx) ? null : (entries[idx] ?? null);
}

async function runGuidedSearch(
  entries: SearchEntry[],
  config: Config,
): Promise<void> {
  const queryInput = await text({
    message: "Search skills:",
    placeholder: "type to filter by name, description…",
  });

  if (isCancel(queryInput)) process.exit(2);

  const q = String(queryInput).trim();
  const filtered = q ? filterEntries(entries, q) : entries;

  if (filtered.length === 0) {
    process.stdout.write(`No skills found matching '${q}'.\n`);
    process.exit(0);
  }

  const result = await select({
    message: "Select a skill to install:",
    options: filtered.map((entry, i) => ({
      value: i,
      label: entry.name,
      hint: `[${entry.source}] ${entry.description}`,
    })),
  });

  if (isCancel(result)) process.exit(2);

  const chosen = filtered[result as number];
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
