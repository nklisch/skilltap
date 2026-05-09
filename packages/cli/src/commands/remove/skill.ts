import {
  discoverSkills,
  type InstalledSkill,
  loadSkillState,
  removeAnySkill,
  removeSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { sendEvent, telemetryBase } from "../../telemetry";
import { confirmRemove, selectSkillsToRemove } from "../../ui/prompts";
import { setupRemoveContext } from "./shared";

export const skillRemoveCommand = defineCommand({
  meta: {
    name: "skill",
    description: "Remove an installed skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name(s) of skills to remove",
      required: false,
    },
    scope: {
      type: "string",
      description:
        "Install scope to remove from (project | global). Defaults to smart-scope (project inside a git repo, global otherwise).",
      valueHint: "project|global",
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const {
      out,
      config,
      projectRoot,
      scope: ctxScope,
      scopeProvided,
    } = await setupRemoveContext(args);
    const globalResult = await loadSkillState();
    if (!globalResult.ok) {
      out.error(globalResult.error.message);
      process.exit(1);
    }
    const projectResult = projectRoot
      ? await loadSkillState(projectRoot)
      : null;
    const allSkills: InstalledSkill[] = [
      ...globalResult.value.skills,
      ...(projectResult?.ok ? projectResult.value.skills : []),
    ];

    let skillsToRemove: InstalledSkill[];

    if (!args.name) {
      if (allSkills.length === 0) {
        out.error("No skills installed.");
        process.exit(1);
      }
      const selected = await selectSkillsToRemove(allSkills);
      const selectedKeys = new Set(selected);
      skillsToRemove = allSkills.filter((s) =>
        selectedKeys.has(`${s.name}:${s.scope}`),
      );
    } else {
      const names = [...new Set((args._ as string[] | undefined) ?? [])];
      skillsToRemove = [];
      for (const name of names) {
        const skill = allSkills.find((s) => s.name === name);
        if (!skill) {
          const discoverResult = await discoverSkills({ unmanagedOnly: true });
          if (discoverResult.ok) {
            const discovered = discoverResult.value.skills.find(
              (s) => s.name === name,
            );
            if (discovered) {
              if (!args.yes) {
                const confirmed = await confirmRemove(name);
                if (confirmed === false) process.exit(2);
              }
              const p = out.progress(`Removing ${name}...`);
              const rmResult = await removeAnySkill({
                skill: discovered,
                removeAll: true,
              });
              if (!rmResult.ok) {
                p.fail("Failed.");
                out.error(rmResult.error.message, rmResult.error.hint);
                process.exit(1);
              }
              p.succeed("Removed.");
              out.success(`Removed ${name}`);
              sendEvent(config, "remove", {
                ...telemetryBase(),
                success: true,
              });
              return;
            }
          }

          out.error(
            `Skill '${name}' is not installed`,
            "Run 'skilltap status' to see installed skills.",
          );
          process.exit(1);
        }
        skillsToRemove.push(skill);
      }
    }

    const scopeOf = (skill: InstalledSkill): "global" | "project" | "linked" =>
      scopeProvided
        ? ctxScope
        : (skill.scope as "global" | "project" | "linked");

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
    const p = out.progress(`Removing ${label}...`);

    for (const skill of skillsToRemove) {
      const result = await removeSkill(skill.name, {
        scope: scopeOf(skill),
        projectRoot: scopeOf(skill) === "project" ? projectRoot : undefined,
        onOrphanRemoved(name) {
          p.update(
            `Note: "${name}" directory was already missing — cleaning up record only.`,
          );
        },
      });
      if (!result.ok) {
        p.fail("Failed.");
        sendEvent(config, "remove", {
          ...telemetryBase(),
          success: false,
          error_category: result.error.constructor.name,
          scope: scopeOf(skill),
        });
        out.error(result.error.message, result.error.hint);
        process.exit(1);
      }
    }

    sendEvent(config, "remove", { ...telemetryBase(), success: true });
    p.succeed("Removed.");
    if (skillsToRemove.length === 1) {
      // biome-ignore lint/style/noNonNullAssertion: length === 1 guard
      out.success(`Removed ${skillsToRemove[0]!.name}`);
    } else {
      out.success(`Removed ${skillsToRemove.length} skills`);
    }
  },
});
