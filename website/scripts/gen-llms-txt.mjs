import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const siteUrl = "https://skilltap.dev";
const pages = [
  ["guide/what-is-skilltap.md", `${siteUrl}/guide/what-is-skilltap`],
  ["guide/getting-started.md", `${siteUrl}/guide/getting-started`],
  ["guide/managing-environments.md", `${siteUrl}/guide/managing-environments`],
  ["guide/instructions.md", `${siteUrl}/guide/instructions`],
  ["guide/updates.md", `${siteUrl}/guide/updates`],
  ["reference/cli.md", `${siteUrl}/reference/cli`],
  ["reference/harnesses.md", `${siteUrl}/reference/harnesses`],
  ["reference/state.md", `${siteUrl}/reference/state`],
];

function stripFrontmatter(content) {
  if (!content.startsWith("---\n")) return content;
  const end = content.indexOf("\n---\n", 4);
  return end === -1 ? content : content.slice(end + 5).trimStart();
}

const sections = [
  `# skilltap — Complete Documentation\n\n> Source: ${siteUrl}/llms-full.txt\n> Generated from the current website documentation.\n`,
];

for (const [file, url] of pages) {
  const raw = await readFile(join(websiteDir, file), "utf8");
  sections.push(`---\n\n<!-- Source: ${url} -->\n\n${stripFrontmatter(raw).trimEnd()}\n`);
}

const output = sections.join("\n");
await writeFile(join(websiteDir, "public", "llms-full.txt"), output);
console.log(`Generated llms-full.txt (${pages.length} pages)`);
