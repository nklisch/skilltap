const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";
const DIM = "\x1b[2m";
const RED = "\x1b[31m";
const YELLOW = "\x1b[33m";
const GREEN = "\x1b[32m";

export const ansi = {
  bold: (s: string) => `${BOLD}${s}${RESET}`,
  dim: (s: string) => `${DIM}${s}${RESET}`,
  red: (s: string) => `${RED}${s}${RESET}`,
  yellow: (s: string) => `${YELLOW}${s}${RESET}`,
  green: (s: string) => `${GREEN}${s}${RESET}`,
};

export function termWidth(): number {
  return process.stdout.columns ?? 80;
}

export function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return `${s.slice(0, max - 1)}…`;
}

export function table(
  rows: string[][],
  opts?: { header?: string[] },
): string {
  const allRows = opts?.header ? [opts.header, ...rows] : rows;
  if (allRows.length === 0) return "";

  const numCols = Math.max(...allRows.map((r) => r.length));
  const colWidths: number[] = Array.from({ length: numCols }, (_, i) =>
    Math.max(...allRows.map((r) => (r[i] ?? "").length)),
  );

  const lines: string[] = [];
  allRows.forEach((row, rowIdx) => {
    const cells = Array.from(
      { length: numCols },
      (_, i) => (row[i] ?? "").padEnd(colWidths[i] ?? 0),
    );
    lines.push(`  ${cells.join("  ")}`);
    if (rowIdx === 0 && opts?.header) {
      const sep = colWidths.map((w) => "─".repeat(w)).join("  ");
      lines.push(`  ${ansi.dim(sep)}`);
    }
  });

  return lines.join("\n");
}

export function errorLine(msg: string, hint?: string): void {
  process.stderr.write(`${ansi.red("error")}: ${msg}\n`);
  if (hint) process.stderr.write(`  ${ansi.dim("hint")}: ${hint}\n`);
}

export function successLine(msg: string): void {
  process.stdout.write(`${ansi.green("✓")} ${msg}\n`);
}
