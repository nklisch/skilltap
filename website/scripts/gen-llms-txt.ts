#!/usr/bin/env bun
/**
 * Generates llms-full.txt — all documentation pages concatenated as Markdown.
 * AI coding assistants (Cursor, Claude Code, Copilot) ingest this for bulk context.
 * Run before vitepress build.
 */

import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const websiteDir = join(__dirname, "..");
const publicDir = join(websiteDir, "public");

const SITE_URL = "https://skilltap.dev";

// Pages in canonical sidebar order
const PAGES: Array<{ file: string; url: string }> = [
  { file: "guide/what-is-skilltap.md", url: `${SITE_URL}/guide/what-is-skilltap` },
  { file: "guide/getting-started.md", url: `${SITE_URL}/guide/getting-started` },
  { file: "guide/installation.md", url: `${SITE_URL}/guide/installation` },
  { file: "guide/installing-skills.md", url: `${SITE_URL}/guide/installing-skills` },
  { file: "guide/creating-skills.md", url: `${SITE_URL}/guide/creating-skills` },
  { file: "guide/taps.md", url: `${SITE_URL}/guide/taps` },
  { file: "guide/security.md", url: `${SITE_URL}/guide/security` },
  { file: "guide/configuration.md", url: `${SITE_URL}/guide/configuration` },
  { file: "guide/doctor.md", url: `${SITE_URL}/guide/doctor` },
  { file: "guide/shell-completions.md", url: `${SITE_URL}/guide/shell-completions` },
  { file: "reference/cli.md", url: `${SITE_URL}/reference/cli` },
  { file: "reference/skill-format.md", url: `${SITE_URL}/reference/skill-format` },
  { file: "reference/tap-format.md", url: `${SITE_URL}/reference/tap-format` },
  { file: "reference/config-options.md", url: `${SITE_URL}/reference/config-options` },
];

function stripFrontmatter(content: string): string {
  if (!content.startsWith("---\n")) return content;
  const end = content.indexOf("\n---\n", 4);
  if (end === -1) return content;
  return content.slice(end + 5).trimStart();
}

const sections: string[] = [
  `# skilltap — Complete Documentation

> Source: ${SITE_URL}/llms-full.txt
> Generated: ${new Date().toISOString().slice(0, 10)}
>
> This file contains the complete skilltap documentation as a single Markdown document
> for AI coding assistant ingestion. For a navigable index see ${SITE_URL}/llms.txt.

`,
];

for (const { file, url } of PAGES) {
  const filePath = join(websiteDir, file);
  const raw = await Bun.file(filePath).text();
  const content = stripFrontmatter(raw);
  sections.push(`---\n\n<!-- Source: ${url} -->\n\n${content.trimEnd()}\n`);
}

const output = sections.join("\n");
await Bun.write(join(publicDir, "llms-full.txt"), output);

const kb = (output.length / 1024).toFixed(1);
console.log(`✓ Generated llms-full.txt (${kb} KB, ${PAGES.length} pages)`);
