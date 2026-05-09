import type {
  Lockfile,
  ManifestEntry,
  ProjectManifest,
} from "../manifest/schemas";
import type { InstalledSkill } from "../schemas/installed";
import type { PluginRecord } from "../schemas/plugins";
import type { State, StoredMcpStandalone } from "../state/schema";
import type { DriftItem, DriftReport, DriftTarget } from "./types";

interface NormalizedEntry {
  source: string;
  ref?: string;
  range: string;
}

// Translate a ManifestEntry (string range or inline-table) into a normalized
// {source, ref?, range} triple keyed by the manifest key (the source string).
function normalizeManifestEntry(
  source: string,
  value: ManifestEntry,
): NormalizedEntry {
  if (typeof value === "string") {
    return { source, range: value };
  }
  return { source, ref: value.ref, range: "*" };
}

function normalizeManifestTable(
  table: Record<string, ManifestEntry>,
): Map<string, NormalizedEntry> {
  const map = new Map<string, NormalizedEntry>();
  for (const [source, value] of Object.entries(table)) {
    map.set(source, normalizeManifestEntry(source, value));
  }
  return map;
}

function normalizeStateSkill(skill: InstalledSkill): {
  source: string | null;
  ref: string | null;
  sha: string | null;
} {
  return { source: skill.repo, ref: skill.ref, sha: skill.sha };
}

function normalizeStatePlugin(plugin: PluginRecord): {
  source: string | null;
  ref: string | null;
  sha: string | null;
} {
  return { source: plugin.repo, ref: plugin.ref, sha: plugin.sha };
}

interface LockEntryNormalized {
  source: string;
  ref: string;
  sha?: string;
  range: string;
}

function lockfileMap(
  entries: { source: string; ref: string; sha?: string; range: string }[],
): Map<string, LockEntryNormalized> {
  const map = new Map<string, LockEntryNormalized>();
  for (const entry of entries) {
    map.set(entry.source, entry);
  }
  return map;
}

export function detectDrift(
  manifest: ProjectManifest,
  lockfile: Lockfile,
  state: State,
): DriftReport {
  const items: DriftItem[] = [];

  // ── Skills ───────────────────────────────────────────────────────────────
  const manifestSkills = normalizeManifestTable(manifest.skills);
  const lockedSkills = lockfileMap(lockfile.skill);
  const stateSkillsBySource = new Map<
    string,
    ReturnType<typeof normalizeStateSkill>
  >();
  for (const skill of state.skills) {
    const norm = normalizeStateSkill(skill);
    if (norm.source) stateSkillsBySource.set(norm.source, norm);
  }

  for (const [source, declared] of manifestSkills) {
    const installed = stateSkillsBySource.get(source);
    const locked = lockedSkills.get(source);

    if (!installed) {
      items.push({
        kind: "add",
        target: "skill",
        source,
        declared: { ref: declared.ref, range: declared.range },
        locked: locked
          ? { ref: locked.ref, sha: locked.sha, range: locked.range }
          : undefined,
        reason: "declared in manifest, not installed",
      });
      continue;
    }

    if (!locked) {
      items.push({
        kind: "lock-missing",
        target: "skill",
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        reason:
          "no lockfile entry — run `skilltap update` to record the resolved ref",
      });
      continue;
    }

    // Range/ref mismatch between manifest and lockfile (someone edited toml).
    if (declared.range !== locked.range) {
      items.push({
        kind: "ref-mismatch",
        target: "skill",
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "manifest range differs from lockfile range",
      });
      continue;
    }

    // Lock-stale: lockfile sha doesn't match installed sha.
    if (locked.sha && installed.sha && locked.sha !== installed.sha) {
      items.push({
        kind: "lock-stale",
        target: "skill",
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "installed sha differs from locked sha",
      });
    }
  }

  for (const [source, installed] of stateSkillsBySource) {
    if (!manifestSkills.has(source)) {
      items.push({
        kind: "remove",
        target: "skill",
        source,
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        reason: "installed but not declared in manifest",
      });
    }
  }

  for (const [source, locked] of lockedSkills) {
    if (!manifestSkills.has(source) && !stateSkillsBySource.has(source)) {
      items.push({
        kind: "lock-orphan",
        target: "skill",
        source,
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "lockfile entry has no manifest or state reference",
      });
    }
  }

  // ── Plugins ──────────────────────────────────────────────────────────────
  const manifestPlugins = normalizeManifestTable(manifest.plugins);
  const lockedPlugins = lockfileMap(lockfile.plugin);
  const statePluginsBySource = new Map<
    string,
    ReturnType<typeof normalizeStatePlugin>
  >();
  for (const plugin of state.plugins) {
    const norm = normalizeStatePlugin(plugin);
    if (norm.source) statePluginsBySource.set(norm.source, norm);
  }

  applyDriftForTable(
    "plugin",
    manifestPlugins,
    lockedPlugins,
    statePluginsBySource,
    items,
  );

  // ── MCPs ─────────────────────────────────────────────────────────────────
  // MCPs are name-keyed, not source-keyed. Their manifest entries carry an
  // exact `ref` pin (no version range). Drift kinds match skills/plugins.
  detectMcpDrift(manifest, lockfile, state, items);

  return {
    items,
    inSync: items.length === 0,
  };
}

function detectMcpDrift(
  manifest: ProjectManifest,
  lockfile: Lockfile,
  state: State,
  items: DriftItem[],
): void {
  const manifestMcps = new Map<string, { source: string; ref: string }>();
  for (const m of manifest.mcps ?? []) {
    manifestMcps.set(m.name, { source: m.source, ref: m.ref });
  }

  const lockedMcps = new Map<
    string,
    { source: string; ref: string; sha: string }
  >();
  for (const m of lockfile.mcps ?? []) {
    lockedMcps.set(m.name, { source: m.source, ref: m.ref, sha: m.sha });
  }

  const stateMcps = new Map<string, StoredMcpStandalone>();
  for (const s of state.mcpServers) {
    stateMcps.set(s.name, s);
  }

  for (const [name, declared] of manifestMcps) {
    const installed = stateMcps.get(name);
    const locked = lockedMcps.get(name);
    const sourceKey = declared.source;

    if (!installed) {
      items.push({
        kind: "add",
        target: "mcp",
        source: sourceKey,
        declared: { ref: declared.ref, range: declared.ref },
        locked: locked
          ? { ref: locked.ref, sha: locked.sha, range: locked.ref }
          : undefined,
        reason: "declared in manifest, not installed",
      });
      continue;
    }

    if (!locked) {
      items.push({
        kind: "lock-missing",
        target: "mcp",
        source: sourceKey,
        declared: { ref: declared.ref, range: declared.ref },
        installed: { ref: declared.ref },
        reason:
          "no lockfile entry — run `skilltap update` to record the resolved ref",
      });
      continue;
    }

    if (declared.ref !== locked.ref) {
      items.push({
        kind: "ref-mismatch",
        target: "mcp",
        source: sourceKey,
        declared: { ref: declared.ref, range: declared.ref },
        installed: { ref: declared.ref },
        locked: { ref: locked.ref, sha: locked.sha, range: locked.ref },
        reason: "manifest ref differs from lockfile ref",
      });
    }
  }

  for (const [name, installed] of stateMcps) {
    if (!manifestMcps.has(name)) {
      items.push({
        kind: "remove",
        target: "mcp",
        source: installed.source,
        installed: { ref: undefined },
        reason: "installed but not declared in manifest",
      });
    }
  }

  for (const [name, locked] of lockedMcps) {
    if (!manifestMcps.has(name) && !stateMcps.has(name)) {
      items.push({
        kind: "lock-orphan",
        target: "mcp",
        source: locked.source,
        locked: { ref: locked.ref, sha: locked.sha, range: locked.ref },
        reason: "lockfile entry has no manifest or state reference",
      });
    }
  }
}

function applyDriftForTable(
  target: DriftTarget,
  manifestMap: Map<string, NormalizedEntry>,
  lockedMap: Map<string, LockEntryNormalized>,
  stateMap: Map<
    string,
    { source: string | null; ref: string | null; sha: string | null }
  >,
  items: DriftItem[],
): void {
  for (const [source, declared] of manifestMap) {
    const installed = stateMap.get(source);
    const locked = lockedMap.get(source);

    if (!installed) {
      items.push({
        kind: "add",
        target,
        source,
        declared: { ref: declared.ref, range: declared.range },
        locked: locked
          ? { ref: locked.ref, sha: locked.sha, range: locked.range }
          : undefined,
        reason: "declared in manifest, not installed",
      });
      continue;
    }

    if (!locked) {
      items.push({
        kind: "lock-missing",
        target,
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        reason:
          "no lockfile entry — run `skilltap update` to record the resolved ref",
      });
      continue;
    }

    if (declared.range !== locked.range) {
      items.push({
        kind: "ref-mismatch",
        target,
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "manifest range differs from lockfile range",
      });
      continue;
    }

    if (locked.sha && installed.sha && locked.sha !== installed.sha) {
      items.push({
        kind: "lock-stale",
        target,
        source,
        declared: { ref: declared.ref, range: declared.range },
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "installed sha differs from locked sha",
      });
    }
  }

  for (const [source, installed] of stateMap) {
    if (!manifestMap.has(source)) {
      items.push({
        kind: "remove",
        target,
        source,
        installed: {
          ref: installed.ref ?? undefined,
          sha: installed.sha ?? undefined,
        },
        reason: "installed but not declared in manifest",
      });
    }
  }

  for (const [source, locked] of lockedMap) {
    if (!manifestMap.has(source) && !stateMap.has(source)) {
      items.push({
        kind: "lock-orphan",
        target,
        source,
        locked: { ref: locked.ref, sha: locked.sha, range: locked.range },
        reason: "lockfile entry has no manifest or state reference",
      });
    }
  }
}
