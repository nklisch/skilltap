import { isCancel, multiselect } from "@clack/prompts";
import { adoptSkill, discoverSkills, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";
import {
  parseAlsoFlag,
  resolveScope,
  tryFindProjectRoot,
} from "../../ui/resolve";

export default defineCommand({
  meta: {
    name: "adopt",
    description: "Adopt unmanaged skills into skilltap management",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name(s) to adopt",
      required: false,
    },
    global: {
      type: "boolean",
      description: "Adopt into global scope",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Adopt into project scope",
      default: false,
    },
    "track-in-place": {
      type: "boolean",
      description: "Track at current location instead of moving",
      default: false,
    },
    also: {
      description: "Also symlink to agent-specific directory",
      valueHint: "agent",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scan",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept all prompts",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: false, quiet: false });
    const configResult = await loadConfig();

    const projectRoot = await tryFindProjectRoot();

    // Discover unmanaged skills
    const discoverOpts = args.global
      ? { global: true as const, unmanagedOnly: true, projectRoot }
      : args.project
        ? { project: true as const, unmanagedOnly: true, projectRoot }
        : { unmanagedOnly: true, projectRoot };

    const discoverResult = await discoverSkills(discoverOpts);

    if (!discoverResult.ok) {
      out.error(discoverResult.error.message, discoverResult.error.hint);
      process.exit(1);
    }

    const unmanaged = discoverResult.value.skills;

    // Collect skill names to adopt
    let namesToAdopt: string[];

    if (args.name) {
      namesToAdopt = [...new Set([args.name, ...(args._ as string[])])];
    } else {
      if (unmanaged.length === 0) {
        out.info("No unmanaged skills found.");
        return;
      }

      const selected = await multiselect({
        message: "Which unmanaged skills to adopt?",
        options: unmanaged.map((s) => ({
          value: s.name,
          label: s.name,
          hint: s.locations[0]?.path,
        })),
        required: true,
      });

      if (isCancel(selected)) process.exit(130);
      namesToAdopt = selected as string[];
    }

    // Find the discovered skill records for each name
    const skillsToAdopt = namesToAdopt.map((name) => {
      const skill = unmanaged.find((s) => s.name === name);
      if (!skill) {
        out.error(
          `Unmanaged skill '${name}' not found.`,
          "Run 'skilltap skills --unmanaged' to see unmanaged skills.",
        );
        process.exit(1);
      }
      return skill;
    });

    const { scope, projectRoot: resolvedProjectRoot } = await resolveScope(
      args,
      configResult.ok ? configResult.value : undefined,
    );
    const also = parseAlsoFlag(
      args.also,
      configResult.ok ? configResult.value : undefined,
    );
    const mode = args["track-in-place"] ? "track-in-place" : "move";

    for (const skill of skillsToAdopt) {
      const result = await adoptSkill(skill, {
        mode,
        scope,
        projectRoot: resolvedProjectRoot,
        also,
        skipScan: args["skip-scan"],
        onWarnings: args.yes
          ? undefined
          : async (warnings, skillName) => {
              process.stderr.write(
                `\nwarning: Security warnings for '${skillName}':\n`,
              );
              for (const w of warnings) {
                process.stderr.write(`  ${w.file}: ${w.category}\n`);
              }
              // In interactive mode without --yes, auto-proceed (warnings were shown)
              return true;
            },
      });

      if (!result.ok) {
        out.error(result.error.message, result.error.hint);
        process.exit(1);
      }

      const { record } = result.value;
      const destPath = record.path ?? `~/.agents/skills/${skill.name}`;

      out.success(`Adopted ${skill.name} → ${destPath}`);
      for (const agent of also) {
        out.success(`  Also linked for ${agent}`);
      }
    }
  },
});
