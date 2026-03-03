/**
 * Parse YAML-style frontmatter between leading --- delimiters.
 * Returns null if no frontmatter found.
 */
export function parseSkillFrontmatter(
  content: string,
): Record<string, unknown> | null {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return null;
  // biome-ignore lint/style/noNonNullAssertion: match[1] is defined because the regex has a capturing group
  const block = match[1]!;
  const data: Record<string, unknown> = {};
  for (const line of block.split("\n")) {
    const sep = line.indexOf(":");
    if (sep === -1) continue;
    const key = line.slice(0, sep).trim();
    if (!key) continue;
    const raw = line.slice(sep + 1).trim();
    if (raw === "true") data[key] = true;
    else if (raw === "false") data[key] = false;
    else if (raw !== "" && !Number.isNaN(Number(raw))) data[key] = Number(raw);
    else data[key] = raw;
  }
  return data;
}
