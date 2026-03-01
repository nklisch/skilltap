import {
  cancel,
  confirm,
  isCancel,
  multiselect,
  select,
} from "@clack/prompts";
import type { ScannedSkill } from "@skilltap/core";

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

export async function confirmRemove(
  name: string,
): Promise<boolean | symbol> {
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
