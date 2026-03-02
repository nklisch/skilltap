export interface NpmTemplateOpts {
  name: string;
  description: string;
  license: string;
  author: string;
}

export function npmTemplate(opts: NpmTemplateOpts): Record<string, string> {
  const license = opts.license !== "None" ? `\nlicense: ${opts.license}` : "";
  const skillMd = `---
name: ${opts.name}
description: ${opts.description}${license}
metadata:
  author: ${opts.author}
  version: "0.1.0"
---

## Instructions

Describe what this skill does and when to use it.

## Rules

- Add rules for the agent to follow
`;

  const packageJson = JSON.stringify(
    {
      name: opts.name,
      version: "0.1.0",
      description: opts.description,
      keywords: ["agent-skill"],
      license: opts.license !== "None" ? opts.license : "UNLICENSED",
      author: opts.author,
      files: ["SKILL.md", "skills/**"],
      repository: {
        type: "git",
        url: "",
      },
    },
    null,
    2,
  );

  const publishYml = `name: Publish
on:
  release:
    types: [published]
permissions:
  id-token: write
  contents: read
  attestations: write
jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          registry-url: https://registry.npmjs.org
      - run: npm publish --provenance --access public
        env:
          NODE_AUTH_TOKEN: \${{ secrets.NPM_TOKEN }}
      - uses: actions/attest-build-provenance@v2
        with:
          subject-path: SKILL.md
`;

  return {
    "SKILL.md": skillMd,
    "package.json": `${packageJson}\n`,
    ".gitignore": "node_modules/\n.DS_Store\n",
    ".github/workflows/publish.yml": publishYml,
  };
}
