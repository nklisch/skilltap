import type { DiffFileStat, DiffStat } from "@skilltap/core";

export type { DiffFileStat, DiffStat };

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

export function table(rows: string[][], opts?: { header?: string[] }): string {
  const allRows = opts?.header ? [opts.header, ...rows] : rows;
  if (allRows.length === 0) return "";

  const numCols = Math.max(...allRows.map((r) => r.length));
  const colWidths: number[] = Array.from({ length: numCols }, (_, i) =>
    Math.max(...allRows.map((r) => (r[i] ?? "").length)),
  );

  const lines: string[] = [];
  allRows.forEach((row, rowIdx) => {
    const cells = Array.from({ length: numCols }, (_, i) =>
      (row[i] ?? "").padEnd(colWidths[i] ?? 0),
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

/** Format install count as "1.2K" or "1.2M" */
export function formatInstallCount(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1).replace(/\.0$/, "")}M installs`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1).replace(/\.0$/, "")}K installs`;
  return `${count} install${count === 1 ? "" : "s"}`;
}

/** "abc1234 → def5678" using 7-char short SHAs */
export function formatShaChange(from: string, to: string): string {
  return `${from.slice(0, 7)} → ${to.slice(0, 7)}`;
}

/** "(2 files changed, +5 -2)" */
export function formatDiffStatSummary(stat: DiffStat): string {
  const files =
    stat.filesChanged === 1 ? "1 file changed" : `${stat.filesChanged} files changed`;
  const parts = [files];
  if (stat.insertions > 0) parts.push(`+${stat.insertions}`);
  if (stat.deletions > 0) parts.push(`-${stat.deletions}`);
  return `(${parts.join(", ")})`;
}

/** "  M SKILL.md (+5 -2)" or "  A scripts/setup.sh (+18)" or "  D old.sh" */
export function formatDiffFileLine(file: DiffFileStat): string {
  const counts: string[] = [];
  if (file.insertions > 0) counts.push(`+${file.insertions}`);
  if (file.deletions > 0) counts.push(`-${file.deletions}`);
  const countStr = counts.length > 0 ? ` (${counts.join(" ")})` : "";
  return `  ${ansi.dim(file.status)} ${file.path}${countStr}`;
}
