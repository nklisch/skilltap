import type { StaticWarning } from "@skilltap/core";

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
