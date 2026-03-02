export interface BasicTemplateOpts {
  name: string;
  description: string;
  license: string;
  author: string;
}

export function basicTemplate(opts: BasicTemplateOpts): Record<string, string> {
  const license = opts.license !== "None" ? `\nlicense: ${opts.license}` : "";
  const author = opts.author ? `\n# author: ${opts.author}` : "";
  const skillMd = `---
name: ${opts.name}
description: ${opts.description}${license}
---
${author}
## Instructions

Describe what this skill does and when to use it.

## Rules

- Add rules for the agent to follow
`;

  return {
    "SKILL.md": skillMd,
    ".gitignore": "node_modules/\n.DS_Store\n",
  };
}
