/**
 * Parse YAML-style frontmatter between leading --- delimiters.
 * Returns null if no frontmatter found.
 * Supports block scalars: > (folded) and | (literal).
 */
export function parseSkillFrontmatter(
  content: string,
): Record<string, unknown> | null {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return null;
  // biome-ignore lint/style/noNonNullAssertion: match[1] is defined because the regex has a capturing group
  const block = match[1]!;
  const data: Record<string, unknown> = {};
  const lines = block.split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;
    const sep = line.indexOf(":");
    if (sep === -1) { i++; continue; }
    const key = line.slice(0, sep).trim();
    if (!key) { i++; continue; }
    const raw = line.slice(sep + 1).trim();
    if (raw === ">" || raw === "|") {
      const style = raw;
      const parts: string[] = [];
      i++;
      while (i < lines.length) {
        const next = lines[i]!;
        if (next.length > 0 && (next[0] === " " || next[0] === "\t")) {
          parts.push(next.trimStart());
          i++;
        } else break;
      }
      data[key] = style === ">" ? parts.join(" ").trim() : parts.join("\n").trimEnd();
    } else if (raw === "") {
      // Empty value — check if next lines are indented (nested object)
      const nested: Record<string, unknown> = {};
      let hasNested = false;
      i++;
      while (i < lines.length) {
        const next = lines[i]!;
        if (next.length > 0 && (next[0] === " " || next[0] === "\t")) {
          const nsep = next.indexOf(":");
          if (nsep !== -1) {
            const nkey = next.slice(0, nsep).trim();
            const nraw = next.slice(nsep + 1).trim();
            if (nkey) {
              hasNested = true;
              if (nraw === "true") nested[nkey] = true;
              else if (nraw === "false") nested[nkey] = false;
              else if (nraw !== "" && !Number.isNaN(Number(nraw))) nested[nkey] = Number(nraw);
              else nested[nkey] = nraw;
            }
          }
          i++;
        } else break;
      }
      data[key] = hasNested ? nested : "";
    } else {
      if (raw === "true") data[key] = true;
      else if (raw === "false") data[key] = false;
      else if (raw !== "" && !Number.isNaN(Number(raw))) data[key] = Number(raw);
      else data[key] = raw;
      i++;
    }
  }
  return data;
}
