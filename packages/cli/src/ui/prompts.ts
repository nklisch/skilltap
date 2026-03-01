import { cancel, confirm, isCancel, multiselect, select } from "@clack/prompts";
import type { AgentAdapter, ScannedSkill, TapEntry } from "@skilltap/core";

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
