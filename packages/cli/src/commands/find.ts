import {
  isCancel,
  outro,
  S_RADIO_ACTIVE,
  S_RADIO_INACTIVE,
} from "@clack/prompts";
import type { Config, Output, RegistrySearchResult, TapEntry } from "@skilltap/core";
import {
  composePolicy,
  ensureBuiltinTap,
  installSkill,
  isBuiltinTapCloned,
  loadConfig,
  loadTaps,
  resolveRegistries,
  saveConfig,
  searchRegistries,
  searchTaps,
} from "@skilltap/core";
import { defineCommand } from "citty";
import pc from "picocolors";
import { setupOutput } from "../ui/setup";
import {
  ansi,
  formatInstallCount,
  highlightMatches,
  table,
  termWidth,
  truncate,
} from "../ui/format";
import { createInstallCallbacks } from "../ui/install-callbacks";
import { createStepLogger } from "../ui/install-steps";
import { confirmSaveDefault, selectAgents } from "../ui/prompts";
import { resolveScope, resolveSemanticInteractive } from "../ui/resolve";
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
  plugin?: boolean;
};

export default defineCommand({
  meta: {
    name: "find",
    description: "Search taps and skills.sh for skills",
  },
  args: {
    query: {
      type: "positional",
      description: "Search term (matched against name, description, tags)",
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
    const out = setupOutput(args);
    // Combine the first positional with any extra words from args._
    // so "skilltap find git hooks" works without quoting
    const rest = (args as Record<string, unknown>)._ as string[] | undefined;
    const parts = [args.query as string | undefined, ...(rest ?? [])].filter(
      Boolean,
    );
    const query = parts.length > 0 ? parts.join(" ") : undefined;

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    // Ensure built-in tap is cloned before searching
    if (config.builtin_tap !== false) {
      const alreadyCloned = await isBuiltinTapCloned();
      if (!alreadyCloned) {
        await ensureBuiltinTap();
      }
    }

    const isTTY = process.stdout.isTTY === true;
    const wantInteractive = args.interactive || (!query && !args.json && isTTY);

    if (wantInteractive) {
      await runInteractiveSearch(query, args.local, config);
      return;
    }

    // Non-interactive path
    const { filtered, tapEntries } = await search(
      query ?? "",
      args.local,
      config,
      out,
    );

    if (filtered.length === 0 && !query) {
      if (tapEntries.length === 0) {
        out.info(
          "No taps configured. Run 'skilltap tap add <name> <url>' to add one.",
        );
        out.info(
          "Tip: search the skills.sh registry with 'skilltap find <query>'.",
        );
      } else {
        out.info("No skills found.");
      }
      process.exit(0);
    }

    if (filtered.length === 0) {
      out.info(`No skills found matching '${query}'.`);
      process.exit(0);
    }

    if (args.json) {
      out.json(
        filtered.map((e) => ({
          name: e.name,
          description: e.description,
          source: e.source,
          installRef: e.installRef,
          ...(e.preSelectedSkill ? { skill: e.preSelectedSkill } : {}),
          ...(e.installs !== undefined ? { installs: e.installs } : {}),
        })),
      );
      return;
    }

    printTable(filtered, out);
  },
});

// ---------------------------------------------------------------------------
// Search helpers
// ---------------------------------------------------------------------------

async function search(
  query: string,
  local: boolean,
  config: Config,
  out: Output,
): Promise<{ filtered: SearchEntry[]; tapEntries: TapEntry[] }> {
  const registries = local ? [] : resolveRegistries(config);

  const [tapsResult, registrySkills] = await Promise.all([
    loadTaps(),
    query.length >= 2 && registries.length > 0
      ? searchRegistries(query, registries, 20)
      : Promise.resolve([] as RegistrySearchResult[]),
  ]);

  if (!tapsResult.ok) {
    out.error(tapsResult.error.message, tapsResult.error.hint);
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
      preSelectedSkill: skill.name,
      trustLabel: formatTapTrust(skill.trust),
      plugin: skill.plugin || undefined,
    }),
  );

  const filtered = applyFilter(
    tapEntries,
    tapSearchEntries,
    registryEntries,
    query,
  );
  return { filtered, tapEntries };
}

function registryToEntries(results: RegistrySearchResult[]): SearchEntry[] {
  return results.map((r) => {
    const skillName = r.id.startsWith(`${r.source}/`)
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
        preSelectedSkill: skill.name,
        trustLabel: formatTapTrust(skill.trust),
        plugin: skill.plugin || undefined,
      }))
    : tapSearchEntries;

  const filteredRegistry = query
    ? registryEntries.filter((e) => {
        const terms = [
          ...new Set(query.toLowerCase().split(/\s+/).filter(Boolean)),
        ];
        const name = e.name.toLowerCase();
        const desc = e.description.toLowerCase();
        return terms.some((t) => name.includes(t) || desc.includes(t));
      })
    : registryEntries;

  const sortedRegistry = [...filteredRegistry].sort(
    (a, b) => (b.installs ?? 0) - (a.installs ?? 0),
  );

  return [...filteredTaps, ...sortedRegistry];
}

// ---------------------------------------------------------------------------
// Table output
// ---------------------------------------------------------------------------

function printTable(entries: SearchEntry[], out: Output): void {
  const width = termWidth();
  const descWidth = Math.max(20, width - 66);
  const rows = entries.map((e) => [
    (e.plugin ? ansi.dim("[plugin] ") : "") + ansi.bold(e.name),
    truncate(e.description, descWidth),
    e.installs !== undefined
      ? ansi.dim(formatInstallCount(e.installs))
      : (e.trustLabel ?? ""),
    ansi.dim(`[${e.source}]`),
  ]);

  out.raw("\n");
  out.raw(table(rows));
  out.raw("\n\n");
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
          plugin: skill.plugin || undefined,
        }));
      }
      const { filtered } = await search(query, local, config, setupOutput({ json: false, quiet: true }));
      return filtered;
    },
    selector: (entry) => `${entry.name} ${entry.description}`,
    renderItem: (entry, active, positions) => {
      const installs =
        entry.installs !== undefined ? formatInstallCount(entry.installs) : "";
      const source = `[${entry.source}]`;
      const meta = installs ? `${source}  ${installs}` : source;

      // Layout: name  description  [source]  installs
      const rawName = truncate(entry.name, 30);
      const pluginBadge = entry.plugin ? pc.dim("[plugin] ") : "";
      const name =
        pluginBadge +
        (positions ? highlightMatches(rawName, positions) : rawName);

      // Fill description into remaining space between name and meta
      const fixedCols = rawName.length + 2 + meta.length + 2; // name + gaps + meta
      const descSpace = Math.max(0, maxLabelWidth - fixedCols);
      const desc = entry.description
        ? truncate(entry.description, descSpace)
        : "";
      const pad = Math.max(
        1,
        maxLabelWidth - rawName.length - desc.length - meta.length - 2,
      );

      if (active) {
        return `${pc.green(S_RADIO_ACTIVE)} ${name}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(meta)}`;
      }
      return `${pc.dim(S_RADIO_INACTIVE)} ${pc.dim(pluginBadge + rawName)}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(meta)}`;
    },
  });

  if (isCancel(result)) process.exit(130);
  await installChosen(result as SearchEntry, config);
}

// ---------------------------------------------------------------------------
// Install from picker
// ---------------------------------------------------------------------------

async function installChosen(
  chosen: SearchEntry,
  config: Config,
): Promise<void> {
  const out = setupOutput({ json: false, quiet: false });
  const policyResult = composePolicy(config, {});
  if (!policyResult.ok) throw new Error(policyResult.error.message);
  const policy = policyResult.value;
  const { agent } = await resolveSemanticInteractive(
    policy,
    { semantic: false },
    config,
  );

  const { scope, projectRoot } = await resolveScope({}, config);
  let also = config.defaults.also ?? [];

  if (!config.defaults.also.length) {
    const selected = await selectAgents(also);
    also = selected;

    if (also.length) {
      const save = await confirmSaveDefault("Save agent selection as default?");
      if (save) {
        config.defaults.also = also;
        await saveConfig(config);
      }
    }
  }

  const p = out.progress(`Fetching ${chosen.name}…`);

  const steps = createStepLogger(config.verbose);
  const { callbacks, logScanResults } = createInstallCallbacks({
    out,
    progress: p,
    onWarn: config.security.on_warn,
    skipScan: false,
    agent,
    yes: true, // user already picked from the search picker
    source: chosen.name,
    steps,
  });

  const installResult = await installSkill(chosen.installRef, {
    scope,
    projectRoot,
    also,
    skillNames: chosen.preSelectedSkill ? [chosen.preSelectedSkill] : undefined,
    skipScan: false,
    agent,
    semantic: policy.scanMode === "semantic",
    threshold: config.scanner.threshold,
    ...callbacks,
  });

  if (!installResult.ok) {
    p.fail("Failed.");
    out.error(installResult.error.message, installResult.error.hint);
    process.exit(1);
  }

  p.succeed();
  logScanResults();
  for (const record of installResult.value.records) {
    out.success(`Installed ${record.name}`);
  }
  outro("Complete!");
}
