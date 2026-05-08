/**
 * Plugin capture — take ownership of standalone skills/MCPs that overlap with
 * a plugin's components.
 *
 * Capture is **source-aware**:
 *   - "same-source" — plugin's canonical repo equals the standalone's canonical
 *     source. Safe path; flows through the normal confirm callback.
 *   - "cross-source" — different canonical sources, or one side has no recorded
 *     source. Silent-substitution risk; default-aborted in auto-confirm modes,
 *     interactive modes can offer a force-override.
 *
 * Detection (`detectCaptureMatches`) is a pure function. Application
 * (`applyCapture`) is the atomic ownership-transfer step that runs before the
 * plugin's content lands on disk.
 *
 * Spec: `docs/design/plugin-capture.md`.
 */

import { rm } from "node:fs/promises";
import { canonicalizeSourceKey, removeSkillFromManifest } from "../manifest/update";
import { skillDisabledDir } from "../paths";
import type { InstalledSkill } from "../schemas/installed";
import type {
  PluginManifest,
  PluginMcpComponent,
  PluginSkillComponent,
} from "../schemas/plugin";
import type { State, StoredMcpStandalone } from "../state/schema";
import { loadState } from "../state/load";
import { saveState } from "../state/save";
import { removeAgentSymlinks } from "../symlink";
import { err, ok, type Result, UserError } from "../types";
import { parseNamespacedKey, removeMcpServers } from "./mcp-inject";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type CaptureCandidate =
  | {
      kind: "skill";
      /** The plugin component that triggered the match. */
      component: PluginSkillComponent;
      /** The standalone record that will be released. */
      standalone: InstalledSkill;
    }
  | {
      kind: "mcp";
      component: PluginMcpComponent;
      standalone: StoredMcpStandalone;
      /**
       * Server name parsed out of the namespaced standalone key
       * (i.e., `parseNamespacedKey(standalone.name).serverName`).
       * Cached so callers and the UI need not re-parse.
       */
      serverName: string;
    };

export interface CaptureBucket {
  skills: Extract<CaptureCandidate, { kind: "skill" }>[];
  mcpServers: Extract<CaptureCandidate, { kind: "mcp" }>[];
}

export interface CaptureMatches {
  /** Standalone canonicalizes to plugin's canonical repo. Safe-path captures. */
  sameSource: CaptureBucket;
  /** Different canonical sources, or one side lacks a recorded source. */
  crossSource: CaptureBucket;
  sameSourceTotal: number;
  crossSourceTotal: number;
  /** sameSourceTotal + crossSourceTotal. */
  total: number;
}

export interface ApplyCaptureOptions {
  scope: "global" | "project";
  projectRoot?: string;
  /** Plugin name — used to namespace replacement keys and target manifest. */
  pluginName: string;
}

export interface ApplyCaptureResult {
  /** Skill names released from state.skills[]. */
  capturedSkills: string[];
  /** Server names released from state.mcpServers[] (parsed serverName, not full key). */
  capturedMcpServers: string[];
  /** Agent ids whose MCP config files had standalone keys removed. */
  prunedAgents: string[];
}

// ---------------------------------------------------------------------------
// Detection (Unit 1)
// ---------------------------------------------------------------------------

/**
 * The MCP standalone's `source` field is the original user-passed string and
 * may carry an `mcp:` prefix (`mcp:user/repo`, `mcp:github:alice/x`). Strip it
 * before canonicalizing so a `mcp:` standalone and a bare plugin source compare
 * equal when they refer to the same upstream repo.
 */
function canonicalMcpSource(source: string): string {
  const stripped = source.startsWith("mcp:") ? source.slice(4) : source;
  return canonicalizeSourceKey(stripped);
}

function emptyBucket(): CaptureBucket {
  return { skills: [], mcpServers: [] };
}

function bucketTotal(bucket: CaptureBucket): number {
  return bucket.skills.length + bucket.mcpServers.length;
}

/**
 * Pure function — finds every standalone in `state` whose name (skill) or
 * parsed server-name (mcp) collides with a component declared by `manifest`,
 * then partitions matches by source provenance.
 *
 * Does NOT mutate state. Filtering by scope is the caller's responsibility —
 * `state` should already be the scope-correct state.json.
 */
export function detectCaptureMatches(
  state: State,
  manifest: PluginManifest,
  pluginRepo: string | null,
): CaptureMatches {
  const sameSource = emptyBucket();
  const crossSource = emptyBucket();

  const canonicalPlugin = pluginRepo ? canonicalizeSourceKey(pluginRepo) : null;

  // ---- Skills ----
  const skillComponents = manifest.components.filter(
    (c): c is PluginSkillComponent => c.type === "skill",
  );

  for (const component of skillComponents) {
    const match = state.skills.find((s) => s.name === component.name);
    if (!match) continue;

    const canonicalStandalone = match.repo
      ? canonicalizeSourceKey(match.repo)
      : null;
    const isSameSource =
      canonicalPlugin !== null &&
      canonicalStandalone !== null &&
      canonicalPlugin === canonicalStandalone;

    const candidate = {
      kind: "skill" as const,
      component,
      standalone: match,
    };

    if (isSameSource) {
      sameSource.skills.push(candidate);
    } else {
      crossSource.skills.push(candidate);
    }
  }

  // ---- MCP servers ----
  const mcpComponents = manifest.components.filter(
    (c): c is PluginMcpComponent => c.type === "mcp",
  );

  for (const component of mcpComponents) {
    for (const stored of state.mcpServers) {
      const parsed = parseNamespacedKey(stored.name);
      if (!parsed) continue; // legacy / malformed key — skip silently

      if (parsed.serverName !== component.server.name) continue;

      const canonicalStandalone = stored.source
        ? canonicalMcpSource(stored.source)
        : null;
      const isSameSource =
        canonicalPlugin !== null &&
        canonicalStandalone !== null &&
        canonicalPlugin === canonicalStandalone;

      const candidate = {
        kind: "mcp" as const,
        component,
        standalone: stored,
        serverName: parsed.serverName,
      };

      if (isSameSource) {
        sameSource.mcpServers.push(candidate);
      } else {
        crossSource.mcpServers.push(candidate);
      }
    }
  }

  const sameSourceTotal = bucketTotal(sameSource);
  const crossSourceTotal = bucketTotal(crossSource);

  return {
    sameSource,
    crossSource,
    sameSourceTotal,
    crossSourceTotal,
    total: sameSourceTotal + crossSourceTotal,
  };
}

/** Concatenates two buckets. Used after a force-override decision. */
export function mergeBuckets(a: CaptureBucket, b: CaptureBucket): CaptureBucket {
  return {
    skills: [...a.skills, ...b.skills],
    mcpServers: [...a.mcpServers, ...b.mcpServers],
  };
}

// ---------------------------------------------------------------------------
// Cross-source error hint
// ---------------------------------------------------------------------------

/**
 * Builds a multi-line resolution hint listing every cross-source candidate's
 * standalone source vs. the incoming plugin source. Used in the `UserError`
 * thrown when a cross-source conflict is hit without an `onCaptureConflict`
 * callback (or when the callback returns `"abort"`).
 */
export function buildCrossSourceHint(
  crossSource: CaptureBucket,
  pluginRepo: string | null,
): string {
  const lines: string[] = [];
  const pluginLabel = pluginRepo ? canonicalizeSourceKey(pluginRepo) : "(no repo)";

  for (const c of crossSource.skills) {
    const standaloneLabel = c.standalone.repo
      ? canonicalizeSourceKey(c.standalone.repo)
      : c.standalone.scope === "linked"
        ? `linked at ${c.standalone.path ?? "(unknown path)"}`
        : "(no recorded source)";
    lines.push(
      `  • skill "${c.component.name}": standalone=${standaloneLabel}, plugin=${pluginLabel}`,
    );
  }

  for (const c of crossSource.mcpServers) {
    const standaloneLabel = c.standalone.source
      ? canonicalMcpSource(c.standalone.source)
      : "(no recorded source)";
    lines.push(
      `  • mcp "${c.serverName}": standalone=${standaloneLabel}, plugin=${pluginLabel}`,
    );
  }

  lines.push("");
  lines.push(
    "Run `skilltap remove <name>` on each conflicting standalone, or run the install",
  );
  lines.push(
    "interactively (without --agent) to choose force-override per conflict.",
  );

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Apply (Unit 2)
// ---------------------------------------------------------------------------

/**
 * Atomically transfer ownership of the candidates in `bucket` to the plugin.
 *
 * Provenance-agnostic: callers that want to capture cross-source matches merge
 * them into the bucket via `mergeBuckets` before calling here.
 *
 * Steps:
 *   1. Load current state for the given scope.
 *   2. Filter captured skills out of `state.skills[]`.
 *   3. Filter captured MCPs out of `state.mcpServers[]`.
 *   4. Save state once.
 *   5. Remove agent symlinks for each captured skill's `also` list.
 *   6. Delete `.disabled/<name>` dirs for any captured skill that was disabled.
 *   7. Remove standalone MCP keys from each captured MCP's `targets[]` agent
 *      configs (one `removeMcpServers` call per distinct slug).
 *   8. Remove captured skills from `skilltap.toml` (project scope only).
 *
 * Steps 5–8 are best-effort — state is the source of truth; downstream-effect
 * failures don't roll back state. Only state-save failure surfaces as Result.err.
 *
 * Idempotent: calling with an empty bucket is a no-op (no I/O).
 */
export async function applyCapture(
  bucket: CaptureBucket,
  options: ApplyCaptureOptions,
): Promise<Result<ApplyCaptureResult, UserError>> {
  if (bucketTotal(bucket) === 0) {
    return ok({
      capturedSkills: [],
      capturedMcpServers: [],
      prunedAgents: [],
    });
  }

  const { scope, projectRoot } = options;
  const stateRoot = scope === "project" ? projectRoot : undefined;

  // 1. Load state.
  const stateResult = await loadState(stateRoot);
  if (!stateResult.ok) {
    return err(
      new UserError(
        `Failed to load state during capture: ${stateResult.error.message}`,
      ),
    );
  }
  const state = stateResult.value;

  // 2 & 3. Filter captured records out.
  const capturedSkillNames = new Set(
    bucket.skills.map((c) => c.standalone.name),
  );
  const capturedMcpKeys = new Set(
    bucket.mcpServers.map((c) => c.standalone.name),
  );

  const newState: State = {
    ...state,
    skills: state.skills.filter((s) => !capturedSkillNames.has(s.name)),
    mcpServers: state.mcpServers.filter((m) => !capturedMcpKeys.has(m.name)),
  };

  // 4. Save state once.
  const saveResult = await saveState(newState, stateRoot);
  if (!saveResult.ok) {
    return err(
      new UserError(
        `Failed to save state during capture: ${saveResult.error.message}`,
      ),
    );
  }

  // ---- Best-effort downstream effects (failures logged but don't fail capture) ----

  // 5. Remove agent symlinks for captured skills.
  for (const c of bucket.skills) {
    const skill = c.standalone;
    if (skill.also.length === 0) continue;
    // Linked skills' symlink scope follows the install scope, not the record's
    // "linked" placeholder — treat them like the plugin's scope (`options.scope`).
    const symlinkScope = skill.scope === "linked" ? scope : skill.scope;
    await removeAgentSymlinks(
      skill.name,
      skill.also,
      symlinkScope,
      projectRoot,
    ).catch(() => {
      // best effort
    });
  }

  // 6. Disabled-skill dir cleanup.
  for (const c of bucket.skills) {
    const skill = c.standalone;
    if (skill.active !== false) continue;
    const dir = skillDisabledDir(skill.name, scope, projectRoot);
    await rm(dir, { recursive: true, force: true }).catch(() => {
      // best effort
    });
  }

  // 7. Prune standalone MCP keys from agent configs, grouped by their slug.
  // Keys are namespaced as `skilltap:<slug>:<server>`; we removeMcpServers
  // by `pluginName: slug` so the existing prefix-prune logic does the work.
  const prunedAgentsSet = new Set<string>();
  const slugBuckets = new Map<string, Set<string>>(); // slug → union of targets
  for (const c of bucket.mcpServers) {
    const parsed = parseNamespacedKey(c.standalone.name);
    if (!parsed) continue;
    const slug = parsed.pluginName;
    const targets = slugBuckets.get(slug) ?? new Set<string>();
    for (const t of c.standalone.targets) targets.add(t);
    slugBuckets.set(slug, targets);
  }
  for (const [slug, targets] of slugBuckets) {
    if (targets.size === 0) continue;
    const result = await removeMcpServers({
      pluginName: slug,
      agents: Array.from(targets),
      scope,
      projectRoot,
    });
    if (result.ok) {
      for (const a of result.value) prunedAgentsSet.add(a);
    }
    // failures swallowed — doctor will surface dangling keys
  }

  // 8. Manifest cleanup (project scope only).
  if (scope === "project" && projectRoot) {
    for (const c of bucket.skills) {
      if (!c.standalone.repo) continue;
      await removeSkillFromManifest(projectRoot, c.standalone.repo).catch(
        () => {
          // best effort
        },
      );
    }
  }

  return ok({
    capturedSkills: Array.from(capturedSkillNames),
    capturedMcpServers: bucket.mcpServers.map((c) => c.serverName),
    prunedAgents: Array.from(prunedAgentsSet),
  });
}
