import { cancel, isCancel } from "@clack/prompts";
import type { AgentAdapter, InstalledSkill, ScannedSkill, TapEntry } from "@skilltap/core";
import { AGENT_LABELS, detectAgents, VALID_AGENT_IDS } from "@skilltap/core";
import {
  footerConfirm as confirm,
  footerMultiselect as multiselect,
  footerSelect as select,
  footerText as text,
} from "./footer";

export async function selectSkills(
  skills: ScannedSkill[],
): Promise<string[] | symbol> {
  const result = await multiselect({
    message: "Which skills to install?",
    options: skills.map((s) => ({
      value: s.name,
      label: s.name,
      hint: s.description || undefined,
    })),
    required: true,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as string[];
}

export async function selectScope(): Promise<string | symbol> {
  const result = await select({
    message: "Install to:",
    options: [
      { value: "global", label: "Global (~/.agents/skills/)" },
      {
        value: "project",
        label: "Project (.agents/skills/)",
        hint: "recommended",
      },
    ],
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as string;
}

export async function confirmReadyInstall(
  skillNames: string[],
): Promise<boolean | symbol> {
  const label = skillNames.length === 1 ? skillNames[0]! : `${skillNames.length} skills`;
  const result = await confirm({
    message: `Install ${label}?`,
    initialValue: true,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as boolean;
}

export async function confirmInstall(
  skillName: string,
): Promise<boolean | symbol> {
  const result = await confirm({
    message: `Install ${skillName} despite warnings?`,
    initialValue: false,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as boolean;
}

export async function selectTap(
  matches: TapEntry[],
): Promise<TapEntry | symbol> {
  const result = await select({
    message: "Multiple taps contain this skill. Which one?",
    options: matches.map((entry, i) => ({
      value: i,
      label: `[${entry.tapName}] ${entry.skill.name}`,
      hint: entry.skill.description,
    })),
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result as symbol;
  }
  // biome-ignore lint/style/noNonNullAssertion: result is a valid index from the select options
  return matches[result as number]!;
}

export async function selectSkillsToRemove(
  skills: InstalledSkill[],
): Promise<string[] | symbol> {
  const result = await multiselect({
    message: "Which skills to remove?",
    options: skills.map((s) => ({
      value: s.name,
      label: s.name,
      hint: s.scope,
    })),
    required: true,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as string[];
}

export async function confirmRemove(name: string): Promise<boolean | symbol> {
  const result = await confirm({
    message: `Remove ${name}?`,
    initialValue: false,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as boolean;
}

export async function selectAgent(
  agents: AgentAdapter[],
): Promise<AgentAdapter | symbol> {
  const result = await select({
    message: "Which agent CLI should be used for semantic scanning?",
    options: agents.map((agent, i) => ({
      value: i,
      label: agent.name,
      hint: agent.cliName,
    })),
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result as symbol;
  }
  // biome-ignore lint/style/noNonNullAssertion: result is a valid index from the select options
  return agents[result as number]!;
}

export async function offerSemanticScan(): Promise<boolean | symbol> {
  const result = await confirm({
    message: "Run semantic scan?",
    initialValue: true,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as boolean;
}

export async function selectAgents(
  currentSelection: string[],
): Promise<string[] | symbol> {
  const result = await multiselect({
    message: "Which agents should this skill be available to?",
    options: VALID_AGENT_IDS.map((id) => ({
      value: id,
      label: AGENT_LABELS[id] ?? id,
    })),
    initialValues: currentSelection,
    required: false,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as string[];
}

/** Shared scan mode select options — used in both config wizards. */
export const SCAN_MODE_OPTIONS = [
  { value: "static", label: "Static only", hint: "fast, catches common attacks" },
  { value: "semantic", label: "Static + Semantic", hint: "thorough, uses your agent CLI" },
  { value: "off", label: "Off", hint: "not recommended" },
];

export async function confirmSaveDefault(
  message: string,
): Promise<boolean | symbol> {
  const result = await confirm({
    message,
    initialValue: false,
  });
  if (isCancel(result)) {
    cancel("Operation cancelled.");
    return result;
  }
  return result as boolean;
}

/**
 * Config wizard helper: detect agents and let user pick one for semantic scanning.
 * Includes "Other — enter path" option. Returns cliName or absolute path.
 */
export async function selectAgentForConfig(
  currentAgent: string,
): Promise<string> {
  const detected = await detectAgents();
  const options: { value: string; label: string; hint?: string }[] =
    detected.map((a) => ({
      value: a.cliName,
      label: a.name,
      hint: a.cliName,
    }));
  options.push({
    value: "__custom",
    label: "Other — enter path",
  });

  const chosen = await select({
    message: "Which agent CLI for scanning?",
    options,
    initialValue: currentAgent || undefined,
  });
  if (isCancel(chosen)) {
    cancel("Setup cancelled.");
    process.exit(2);
  }

  if (chosen === "__custom") {
    const path = await text({
      message: "Enter path to agent CLI binary:",
      validate(v) {
        if (!v || !v.startsWith("/")) return "Must be an absolute path";
      },
    });
    if (isCancel(path)) {
      cancel("Setup cancelled.");
      process.exit(2);
    }
    return path as string;
  }

  return chosen as string;
}
