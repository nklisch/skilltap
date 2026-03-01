import type { SemanticWarning, StaticWarning } from "@skilltap/core";

export function formatWarnings(
  warnings: StaticWarning[],
  skillName: string,
): string {
  const lines: string[] = [`⚠ Static warnings in ${skillName}:`, ""];

  for (const w of warnings) {
    const lineRef = Array.isArray(w.line)
      ? `L${w.line[0]}-${w.line[1]}`
      : w.line > 0
        ? `L${w.line}`
        : w.file;

    lines.push(`  ${lineRef}: ${w.category}`);

    if (w.visible !== undefined && w.visible !== w.raw) {
      lines.push(`  │ Raw: "${w.raw}"`);
      lines.push(`  │ Visible: "${w.visible}"`);
    } else {
      lines.push(`  │ ${w.raw}`);
    }
    lines.push("");
  }

  return lines.join("\n");
}

export function printWarnings(
  warnings: StaticWarning[],
  skillName: string,
): void {
  process.stderr.write(`${formatWarnings(warnings, skillName)}\n`);
}

export function formatSemanticWarnings(
  warnings: SemanticWarning[],
  skillName: string,
): string {
  const lines: string[] = [`⚠ Semantic warnings in ${skillName}:`, ""];

  for (const w of warnings) {
    const lineRef = `L${w.lineRange[0]}-${w.lineRange[1]}`;
    lines.push(`  ${lineRef} (chunk ${w.chunkIndex}) — risk ${w.score}/10`);

    // Show truncated raw content as a quote
    const rawLines = w.raw.split("\n").slice(0, 3);
    for (const line of rawLines) {
      lines.push(`  │ "${line}"`);
    }
    lines.push(`  │ → ${w.reason}`);
    lines.push("");
  }

  return lines.join("\n");
}

export function printSemanticWarnings(
  warnings: SemanticWarning[],
  skillName: string,
): void {
  process.stderr.write(`${formatSemanticWarnings(warnings, skillName)}\n`);
}
