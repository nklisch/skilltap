import { extname, join } from "node:path";
import type { Result } from "../types";
import { err, ok, ScanError } from "../types";
import {
  detectDangerousPatterns,
  detectHiddenHtmlCss,
  detectInvisibleUnicode,
  detectMarkdownHiding,
  detectObfuscation,
  detectSuspiciousUrls,
  detectTagInjection,
} from "./patterns";

const DETECTORS = [
  detectInvisibleUnicode,
  detectHiddenHtmlCss,
  detectMarkdownHiding,
  detectObfuscation,
  detectSuspiciousUrls,
  detectDangerousPatterns,
  detectTagInjection,
];

export type StaticWarning = {
  file: string;
  line: number | [number, number];
  category: string;
  raw: string;
  visible?: string;
  decoded?: string;
  context?: string[];
};

const DEFAULT_MAX_SIZE = 51200; // 50KB
const MAX_FILE_SIZE = 20480; // 20KB

// Extensions that indicate unexpected binary/archive files
const FLAGGED_EXTENSIONS = new Set([
  ".wasm",
  ".pyc",
  ".class",
  ".zip",
  ".tar",
  ".gz",
]);

/** Returns true if the path is inside a VCS metadata directory (.git/, .svn/, .hg/). */
export function isVcsPath(relPath: string): boolean {
  return (
    relPath.startsWith(".git/") ||
    relPath.startsWith(".svn/") ||
    relPath.startsWith(".hg/")
  );
}

// Binary magic byte signatures
const BINARY_SIGNATURES: Array<{ sig: Uint8Array; category: string }> = [
  { sig: new Uint8Array([0x7f, 0x45, 0x4c, 0x46]), category: "ELF binary" },
  { sig: new Uint8Array([0xce, 0xfa, 0xed, 0xfe]), category: "Mach-O binary" },
  { sig: new Uint8Array([0xcf, 0xfa, 0xed, 0xfe]), category: "Mach-O binary" },
  { sig: new Uint8Array([0x4d, 0x5a]), category: "PE binary" },
  {
    sig: new Uint8Array([0x00, 0x61, 0x73, 0x6d]),
    category: "WebAssembly binary",
  },
];

function hasBinaryMagic(bytes: Uint8Array): string | null {
  for (const { sig, category } of BINARY_SIGNATURES) {
    if (bytes.length >= sig.length) {
      let match = true;
      for (let i = 0; i < sig.length; i++) {
        if (bytes[i] !== sig[i]) {
          match = false;
          break;
        }
      }
      if (match) return category;
    }
  }
  return null;
}

/** Validate and scan a single file for security issues. */
async function scanSingleFile(
  relPath: string,
  dir: string,
  warnings: StaticWarning[],
): Promise<void> {
  const filePath = join(dir, relPath);
  const ext = extname(relPath).toLowerCase();

  if (FLAGGED_EXTENSIONS.has(ext)) {
    warnings.push({
      file: relPath,
      line: 0,
      category: "Unexpected file type",
      raw: `File type ${ext} is unexpected in a skill`,
    });
    return;
  }

  const bunFile = Bun.file(filePath);
  const fileSize = bunFile.size;

  if (fileSize > MAX_FILE_SIZE) {
    warnings.push({
      file: relPath,
      line: 0,
      category: "Large file",
      raw: `File size ${fileSize} bytes exceeds ${MAX_FILE_SIZE} bytes`,
    });
  }

  const bytes = new Uint8Array(await bunFile.arrayBuffer());
  const binaryType = hasBinaryMagic(bytes);
  if (binaryType) {
    warnings.push({
      file: relPath,
      line: 0,
      category: "Binary file",
      raw: `Detected ${binaryType} signature`,
    });
    return;
  }

  let content: string;
  try {
    content = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    warnings.push({
      file: relPath,
      line: 0,
      category: "Binary file",
      raw: "File contains invalid UTF-8 sequences",
    });
    return;
  }

  if (ext === ".js" || ext === ".ts") {
    const singleLongLine = content
      .split("\n")
      .some((line) => line.length > 500);
    if (singleLongLine) {
      warnings.push({
        file: relPath,
        line: 0,
        category: "Minified code",
        raw: "Single line exceeds 500 characters",
      });
    }
  }

  const fileWarnings: StaticWarning[] = [];
  for (const detect of DETECTORS) {
    for (const m of detect(content)) {
      fileWarnings.push({ file: relPath, ...m });
    }
  }

  // Attach ±1 surrounding lines as context for line-level warnings
  if (fileWarnings.length > 0) {
    const fileLines = content.split("\n");
    for (const w of fileWarnings) {
      const lineNum = Array.isArray(w.line) ? w.line[0] : w.line;
      if (lineNum > 0) {
        const ctx: string[] = [];
        if (lineNum > 1) ctx.push(fileLines[lineNum - 2]!);
        ctx.push(fileLines[lineNum - 1]!);
        if (lineNum < fileLines.length) ctx.push(fileLines[lineNum]!);
        w.context = ctx;
      }
    }
  }

  warnings.push(...fileWarnings);
}

export async function scanStatic(
  dir: string,
  opts?: { maxSize?: number },
): Promise<Result<StaticWarning[], ScanError>> {
  const maxSize = opts?.maxSize ?? DEFAULT_MAX_SIZE;
  const warnings: StaticWarning[] = [];

  try {
    const glob = new Bun.Glob("**/*");
    const relPaths: string[] = [];
    for await (const relPath of glob.scan({
      cwd: dir,
      onlyFiles: true,
      dot: true,
    })) {
      if (isVcsPath(relPath)) continue; // Skip VCS metadata directories
      relPaths.push(relPath);
    }

    // Total size check
    let totalSize = 0;
    for (const relPath of relPaths) {
      const filePath = join(dir, relPath);
      const stat = await Bun.file(filePath).size;
      totalSize += stat;
    }
    if (totalSize > maxSize) {
      warnings.push({
        file: ".",
        line: 0,
        category: "Size warning",
        raw: `Total skill size ${totalSize} bytes exceeds limit of ${maxSize} bytes`,
      });
    }

    for (const relPath of relPaths) {
      await scanSingleFile(relPath, dir, warnings);
    }

    return ok(warnings);
  } catch (e) {
    return err(
      new ScanError(
        `Static scan failed: ${e instanceof Error ? e.message : String(e)}`,
      ),
    );
  }
}

/**
 * Scan a unified diff for security issues in added lines only.
 * Parses `+` lines from each file hunk and runs all pattern detectors.
 */
export function scanDiff(diffOutput: string): StaticWarning[] {
  if (!diffOutput.trim()) return [];

  const warnings: StaticWarning[] = [];

  // Per-file: list of { lineNum (in new file), content }
  type AddedLine = { lineNum: number; content: string };
  const fileAdditions = new Map<string, AddedLine[]>();

  let currentFile = "";
  let currentLineNum = 0;

  for (const line of diffOutput.split("\n")) {
    // Track current file from +++ b/ header
    if (line.startsWith("+++ b/")) {
      currentFile = line.slice(6);
      if (!fileAdditions.has(currentFile)) fileAdditions.set(currentFile, []);
      continue;
    }
    // Hunk header: @@ -old +new_start[,count] @@
    if (line.startsWith("@@ ")) {
      const m = /\+(\d+)/.exec(line);
      currentLineNum = m ? parseInt(m[1]!, 10) : 1;
      continue;
    }
    if (!currentFile) continue;

    if (line.startsWith("+") && !line.startsWith("+++")) {
      // biome-ignore lint/style/noNonNullAssertion: currentFile is set, map entry exists
      fileAdditions.get(currentFile)!.push({
        lineNum: currentLineNum,
        content: line.slice(1),
      });
      currentLineNum++;
    } else if (!line.startsWith("-")) {
      // Context line — advance new-file line counter
      currentLineNum++;
    }
  }

  for (const [file, addedLines] of fileAdditions) {
    if (addedLines.length === 0) continue;
    const lineOffset = addedLines[0]?.lineNum ?? 1;
    const content = addedLines.map((l) => l.content).join("\n");

    for (const detect of DETECTORS) {
      for (const m of detect(content)) {
        const adjustLine = (n: number) => lineOffset + n - 1;
        const adjustedLine: number | [number, number] =
          typeof m.line === "number"
            ? adjustLine(m.line)
            : [adjustLine(m.line[0]), adjustLine(m.line[1])];
        warnings.push({ file, ...m, line: adjustedLine });
      }
    }
  }

  return warnings;
}
