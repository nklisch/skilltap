import { spinner } from "@clack/prompts";
import {
  discoverSkills,
  type InstalledSkill,
  loadInstalled,
  removeAnySkill,
  removeMcpInstall,
  removeSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { sendEvent, telemetryBase } from "../../telemetry";
import { agentError, exitWithError } from "../../ui/agent-out";
import { errorLine, successLine } from "../../ui/format";
import { loadPolicyOrExit } from "../../ui/policy";
import { confirmRemove, selectSkillsToRemove } from "../../ui/prompts";
import { tryFindProjectRoot } from "../../ui/resolve";

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove an installed skill",
  },
  args: {
    name: {
      type: "positional",
      description:
        "Name(s) of installed skills to remove (required in agent mode)",
      required: false,
    },
    project: {
      type: "boolean",
      description: "Remove from project scope instead of global",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Remove from global scope instead of project",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
    agent: {
      type: "boolean",
      description: "Run in non-interactive agent mode (also: SKILLTAP_AGENT=1)",
      default: false,
    },
  },
  async run({ args }) {
    const { config, policy } = await loadPolicyOrExit({
      yes: args.yes,
      project: args.project,
      global: args.global,
      agent: args.agent,
    });

    // Phase 35b-2: dispatch mcp:<source> to MCP-only remove path.
    const allInputs = [
      args.name,
      ...((args._ as string[] | undefined) ?? []),
    ].filter((n): n is string => typeof n === "string" && n.length > 0);
    const mcpInputs = allInputs.filter((n) => n.startsWith("mcp:"));
    if (mcpInputs.length > 0) {
      if (mcpInputs.length !== allInputs.length) {
        errorLine(
          "Cannot mix mcp: and regular sources in one remove. Run them separately.",
        );
        process.exit(1);
      }
      const scope = (policy.scope || "project") as "global" | "project";
      const projectRoot =
        scope === "project" ? await tryFindProjectRoot() : undefined;
      let anyFail = false;
      for (const source of mcpInputs) {
        const result = await removeMcpInstall(source, { scope, projectRoot });
        if (!result.ok) {
          errorLine(result.error.message, result.error.hint);
          anyFail = true;
          continue;
        }
        const r = result.value;
        if (policy.agentMode) {
          process.stdout.write(
            `OK: Removed ${r.removed} MCP server${r.removed === 1 ? "" : "s"} from ${source}\n`,
          );
        } else {
          successLine(
            `Removed ${r.removed} MCP server${r.removed === 1 ? "" : "s"} from ${source} (agents: ${r.agents.join(", ")})`,
          );
          for (const name of r.names) {
            successLine(`  • ${name}`);
          }
        }
      }
      if (anyFail) process.exit(1);
      return;
    }

    const projectRoot = await tryFindProjectRoot();
    const globalResult = await loadInstalled();
    if (!globalResult.ok) {
      exitWithError(policy.agentMode, globalResult.error.message);
    }
    const projectResult = projectRoot ? await loadInstalled(projectRoot) : null;
    const allSkills: InstalledSkill[] = [
      ...globalResult.value.skills,
      ...(projectResult?.ok ? projectResult.value.skills : []),
    ];

    let skillsToRemove: InstalledSkill[];

    if (!args.name) {
      if (policy.agentMode) {
        agentError("Provide skill name(s) as arguments.");
        process.exit(1);
      }
      if (allSkills.length === 0) {
        errorLine("No skills installed.");
        process.exit(1);
      }
      const selected = await selectSkillsToRemove(allSkills);
      const selectedKeys = new Set(selected);
      skillsToRemove = allSkills.filter((s) =>
        selectedKeys.has(`${s.name}:${s.scope}`),
      );
    } else {
      const names = [...new Set([args.name, ...(args._ as string[])])];
      skillsToRemove = [];
      for (const name of names) {
        const skill = allSkills.find((s) => s.name === name);
        if (!skill) {
          // Check if it's an unmanaged skill on disk
          const discoverResult = await discoverSkills({ unmanagedOnly: true });
          if (discoverResult.ok) {
            const discovered = discoverResult.value.skills.find(
              (s) => s.name === name,
            );
            if (discovered) {
              // Confirm and remove unmanaged skill
              if (policy.agentMode) {
                const rmResult = await removeAnySkill({
                  skill: discovered,
                  removeAll: true,
                });
                if (!rmResult.ok) {
                  agentError(rmResult.error.message);
                  process.exit(1);
                }
                process.stdout.write(`OK: Removed ${name}\n`);
                sendEvent(config, "remove", {
                  ...telemetryBase(true),
                  success: true,
                });
                return;
              } else {
                if (!args.yes) {
                  const confirmed = await confirmRemove(name);
                  if (confirmed === false) process.exit(2);
                }
                const s = spinner();
                s.start(`Removing ${name}...`);
                const rmResult = await removeAnySkill({
                  skill: discovered,
                  removeAll: true,
                });
                if (!rmResult.ok) {
                  s.stop("Failed.");
                  errorLine(rmResult.error.message, rmResult.error.hint);
                  process.exit(1);
                }
                s.stop("Removed.");
                successLine(`Removed ${name}`);
                sendEvent(config, "remove", {
                  ...telemetryBase(false),
                  success: true,
                });
                return;
              }
            }
          }

          exitWithError(
            policy.agentMode,
            `Skill '${name}' is not installed`,
            "Run 'skilltap skills' to see installed skills.",
          );
        }
        skillsToRemove.push(skill);
      }
    }

    const scopeOf = (skill: InstalledSkill): "global" | "project" | "linked" =>
      args.project
        ? "project"
        : args.global
          ? "global"
          : (skill.scope as "global" | "project" | "linked");

    if (policy.agentMode) {
      for (const skill of skillsToRemove) {
        const result = await removeSkill(skill.name, {
          scope: scopeOf(skill),
          projectRoot: scopeOf(skill) === "project" ? projectRoot : undefined,
          onOrphanRemoved(name) {
            process.stdout.write(
              `note: "${name}" directory was already missing — cleaning up record only.\n`,
            );
          },
        });
        if (!result.ok) {
          sendEvent(config, "remove", {
            ...telemetryBase(true),
            success: false,
            error_category: result.error.constructor.name,
            scope: scopeOf(skill),
          });
          agentError(result.error.message);
          process.exit(1);
        }
        process.stdout.write(`OK: Removed ${skill.name}\n`);
      }
      sendEvent(config, "remove", { ...telemetryBase(true), success: true });
      return;
    }

    // Confirm only when names were given via CLI (multiselect is implicit confirmation)
    if (!args.yes && args.name) {
      const label =
        skillsToRemove.length === 1
          ? // biome-ignore lint/style/noNonNullAssertion: length === 1 guard
            skillsToRemove[0]!.name
          : `${skillsToRemove.length} skills`;
      const confirmed = await confirmRemove(label);
      if (confirmed === false) process.exit(2);
    }

    const label =
      skillsToRemove.length === 1
        ? // biome-ignore lint/style/noNonNullAssertion: length === 1 guard
          skillsToRemove[0]!.name
        : `${skillsToRemove.length} skills`;
    const s = spinner();
    s.start(`Removing ${label}...`);

    for (const skill of skillsToRemove) {
      const result = await removeSkill(skill.name, {
        scope: scopeOf(skill),
        projectRoot: scopeOf(skill) === "project" ? projectRoot : undefined,
        onOrphanRemoved(name) {
          s.message(
            `Note: "${name}" directory was already missing — cleaning up record only.`,
          );
        },
      });
      if (!result.ok) {
        s.stop("Failed.");
        sendEvent(config, "remove", {
          ...telemetryBase(false),
          success: false,
          error_category: result.error.constructor.name,
          scope: scopeOf(skill),
        });
        errorLine(result.error.message, result.error.hint);
        process.exit(1);
      }
    }

    sendEvent(config, "remove", { ...telemetryBase(false), success: true });
    s.stop("Removed.");
    if (skillsToRemove.length === 1) {
      // biome-ignore lint/style/noNonNullAssertion: length === 1 guard
      successLine(`Removed ${skillsToRemove[0]!.name}`);
    } else {
      successLine(`Removed ${skillsToRemove.length} skills`);
    }
  },
});
