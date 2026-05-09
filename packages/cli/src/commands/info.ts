import { lstat } from "node:fs/promises";
import { join } from "node:path";
import type {
  Output,
  PluginRecord,
  SkillRecord,
  StoredComponent,
} from "@skilltap/core";
import {
  AGENT_PATHS,
  ensureBuiltinTap,
  globalBase,
  isBuiltinTapCloned,
  loadConfig,
  loadState,
  loadTaps,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { tryFindProjectRoot } from "../ui/resolve";
import { setupOutput } from "../ui/setup";
import { formatTrustLabel } from "../ui/trust";

export const infoCommand = defineCommand({
  meta: {
    name: "info",
    description: "Show details for a skill, plugin, or MCP server",
  },
  args: {
    name: {
      type: "positional",
      required: true,
      description: "Name of the skill, plugin, or MCP server",
    },
    json: { type: "boolean", default: false, description: "Output as JSON" },
    project: {
      type: "boolean",
      default: false,
      description: "Restrict lookup to project scope",
    },
    global: {
      type: "boolean",
      default: false,
      description: "Restrict lookup to global scope",
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const name = args.name as string;
    const projectRoot = await tryFindProjectRoot();

    // ── Load state ───────────────────────────────────────────────────────────
    const stateResult = await loadState(projectRoot);
    if (!stateResult.ok) {
      out.error(stateResult.error.message, stateResult.error.hint);
      process.exit(1);
    }
    const state = stateResult.value;

    // Also load global state when inside a project (loadState returns project
    // scope; global entries might also match).
    const globalStateResult = projectRoot ? await loadState(undefined) : null;

    const allSkills: SkillRecord[] = [
      ...(globalStateResult?.ok ? globalStateResult.value.skills : []),
      ...state.skills,
    ];
    const allPlugins: (PluginRecord & { _scope: string })[] = [
      ...(globalStateResult?.ok
        ? globalStateResult.value.plugins.map((p) => ({
            ...p,
            _scope: "global",
          }))
        : []),
      ...state.plugins.map((p) => ({ ...p, _scope: "project" })),
    ];
    const allMcps = [
      ...(globalStateResult?.ok ? globalStateResult.value.mcpServers : []),
      ...state.mcpServers,
    ];

    // ── Scope filters ────────────────────────────────────────────────────────
    const scopeFilter = args.global
      ? "global"
      : args.project
        ? "project"
        : null;

    const filteredSkills = scopeFilter
      ? allSkills.filter((s) => s.scope === scopeFilter)
      : allSkills;
    const filteredPlugins = scopeFilter
      ? allPlugins.filter((p) => p._scope === scopeFilter)
      : allPlugins;
    const filteredMcps = scopeFilter
      ? allMcps.filter((m) => m.scope === scopeFilter)
      : allMcps;

    // ── Lookup: prefer plugin > skill > mcp; prefer project scope ────────────
    const pluginMatch = filteredPlugins.find((p) => p.name === name);
    const skillMatch = filteredSkills.find((s) => s.name === name);
    const mcpMatch = filteredMcps.find((m) => m.name === name);

    if (pluginMatch) {
      return renderPluginInfo(pluginMatch, out, args.json as boolean);
    }

    if (skillMatch) {
      return renderSkillInfo(
        skillMatch,
        out,
        args.json as boolean,
        projectRoot,
      );
    }

    if (mcpMatch) {
      return renderMcpInfo(mcpMatch, out, args.json as boolean);
    }

    // ── Not found in state — check taps ──────────────────────────────────────
    const configResult = await loadConfig();
    if (configResult.ok && configResult.value.builtin_tap !== false) {
      const alreadyCloned = await isBuiltinTapCloned();
      if (!alreadyCloned) await ensureBuiltinTap();
    }

    // Check taps
    const tapsResult = await loadTaps();
    if (tapsResult.ok) {
      const tapEntry = tapsResult.value.find((e) => e.skill.name === name);
      if (tapEntry) {
        if (args.json) {
          out.json({
            ...tapEntry.skill,
            tap: tapEntry.tapName,
            status: "available",
          });
          return;
        }
        const tapTrust = tapEntry.skill.trust?.verified
          ? ansi.dim("◆ verified by tap")
          : undefined;
        const rows: [string, string][] = [
          ["name:", ansi.bold(tapEntry.skill.name)],
          ["description:", tapEntry.skill.description || "—"],
          ["status:", ansi.dim("(available)")],
          ["tap:", tapEntry.tapName],
          ["source:", tapEntry.skill.repo],
          [
            "tags:",
            tapEntry.skill.tags.length > 0
              ? tapEntry.skill.tags.join(", ")
              : "—",
          ],
          ...(tapTrust ? [["trust:", tapTrust] as [string, string]] : []),
        ];
        for (const [key, val] of rows) {
          out.raw(`${ansi.dim(key.padEnd(13))} ${val}\n`);
        }
        out.raw(`\nRun 'skilltap install skill ${name}' to install.\n`);
        return;
      }
    }

    out.error(
      `No skill, plugin, or MCP server named "${name}" is installed.`,
      `Run 'skilltap find ${name}' to search taps.`,
    );
    process.exit(1);
  },
});

export default infoCommand;

// ─── Renderers ────────────────────────────────────────────────────────────────

async function renderSkillInfo(
  skill: SkillRecord,
  out: Output,
  json: boolean,
  projectRoot: string | undefined,
): Promise<void> {
  if (json) {
    out.json(skill);
    return;
  }

  const base =
    skill.scope === "project" ? (projectRoot ?? process.cwd()) : globalBase();
  const skillPath = join(base, ".agents", "skills", skill.name);

  const agentStatus = await Promise.all(
    Object.entries(AGENT_PATHS).map(async ([agent, dir]) => {
      const path = join(base, dir, skill.name);
      const exists = await lstat(path)
        .then(() => true)
        .catch(() => false);
      return { agent, exists };
    }),
  );

  const activeAgents = agentStatus.filter((a) => a.exists).map((a) => a.agent);

  const rows: [string, string][] = [
    ["name:", ansi.bold(skill.name)],
    ["description:", skill.description || "—"],
    ["scope:", skill.scope],
    ["source:", skill.repo ?? "local"],
    ["ref:", skill.ref ?? "—"],
    ["sha:", skill.sha ? skill.sha.slice(0, 7) : "—"],
    [
      "trust:",
      skill.trust ? formatTrustLabel(skill.trust) : ansi.dim("○ unverified"),
    ],
    ["path:", skillPath],
    ["agents:", activeAgents.length > 0 ? activeAgents.join(", ") : "none"],
    ["installed:", skill.installedAt],
    ["updated:", skill.updatedAt],
  ];

  if (skill.trust?.tier === "provenance") {
    if (skill.trust.npm) {
      rows.push(["  source:", skill.trust.npm.sourceRepo]);
      if (skill.trust.npm.buildWorkflow)
        rows.push(["  build:", skill.trust.npm.buildWorkflow]);
      if (skill.trust.npm.transparency)
        rows.push(["  log:", skill.trust.npm.transparency]);
    } else if (skill.trust.github) {
      rows.push([
        "  repo:",
        `${skill.trust.github.owner}/${skill.trust.github.repo}`,
      ]);
      if (skill.trust.github.workflow)
        rows.push(["  build:", skill.trust.github.workflow]);
    }
  }

  for (const [key, val] of rows) {
    out.raw(`${ansi.dim(key.padEnd(13))} ${val}\n`);
  }
}

function componentStatusIcon(c: StoredComponent): string {
  return c.active ? ansi.green("✓") : ansi.dim("✗");
}

function renderPluginInfo(
  plugin: PluginRecord,
  out: Output,
  json: boolean,
): void {
  if (json) {
    out.json(plugin);
    return;
  }

  const rows: [string, string][] = [
    ["name:", ansi.bold(plugin.name)],
    ["description:", plugin.description || "—"],
    ["scope:", plugin.scope],
    ["source:", plugin.repo ?? "local"],
    ["format:", plugin.format],
    ["ref:", plugin.ref ?? "—"],
    ["sha:", plugin.sha ? plugin.sha.slice(0, 7) : "—"],
    ["agents:", plugin.also.length > 0 ? plugin.also.join(", ") : "none"],
    ["installed:", plugin.installedAt],
    ["updated:", plugin.updatedAt],
  ];

  for (const [key, val] of rows) {
    out.raw(`${ansi.dim(key.padEnd(13))} ${val}\n`);
  }

  if (plugin.components.length > 0) {
    out.raw("\n");
    const skills = plugin.components.filter((c) => c.type === "skill");
    const mcps = plugin.components.filter((c) => c.type === "mcp");
    const agents = plugin.components.filter((c) => c.type === "agent");

    if (skills.length > 0) {
      out.raw(`${ansi.bold("Skills:")}\n`);
      for (const c of skills) {
        out.raw(`  ${componentStatusIcon(c)} ${c.name}\n`);
      }
    }
    if (mcps.length > 0) {
      out.raw(`${ansi.bold("MCP Servers:")}\n`);
      for (const c of mcps) {
        out.raw(`  ${componentStatusIcon(c)} ${c.name}\n`);
      }
    }
    if (agents.length > 0) {
      out.raw(`${ansi.bold("Agent Definitions:")}\n`);
      for (const c of agents) {
        out.raw(
          `  ${componentStatusIcon(c)} ${c.name} ${ansi.dim(`(${c.type})`)}\n`,
        );
      }
    }
  }
}

function renderMcpInfo(
  mcp: { name: string; source?: string; scope?: string; installedAt?: string },
  out: Output,
  json: boolean,
): void {
  if (json) {
    out.json(mcp);
    return;
  }

  const rows: [string, string][] = [
    ["name:", ansi.bold(mcp.name)],
    ["scope:", mcp.scope ?? "—"],
    ["source:", mcp.source ?? "—"],
    ["installed:", mcp.installedAt ?? "—"],
  ];

  for (const [key, val] of rows) {
    out.raw(`${ansi.dim(key.padEnd(13))} ${val}\n`);
  }
}
