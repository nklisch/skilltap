export interface MultiTemplateOpts {
  description: string;
  license: string;
  author: string;
  skillNames: string[];
}

export function multiTemplate(opts: MultiTemplateOpts): Record<string, string> {
  const files: Record<string, string> = {
    ".gitignore": "node_modules/\n.DS_Store\n",
  };

  for (const skillName of opts.skillNames) {
    const license = opts.license !== "None" ? `\nlicense: ${opts.license}` : "";
    const skillMd = `---
name: ${skillName}
description: Describe what ${skillName} does and when to use it.${license}
metadata:
  author: ${opts.author}
  version: "0.1.0"
---

## Instructions

Describe what this skill does and when to use it.

## Rules

- Add rules for the agent to follow
`;
    files[`.agents/skills/${skillName}/SKILL.md`] = skillMd;
  }

  return files;
}
