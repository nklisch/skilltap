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

export type StaticWarning = {
  file: string;
  line: number | [number, number];
  category: string;
  raw: string;
  visible?: string;
  decoded?: string;
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
      // Skip VCS metadata directories
      if (
        relPath.startsWith(".git/") ||
        relPath.startsWith(".svn/") ||
        relPath.startsWith(".hg/")
      )
        continue;
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
      const filePath = join(dir, relPath);
      const ext = extname(relPath).toLowerCase();

      // Flag unexpected file types by extension
      if (FLAGGED_EXTENSIONS.has(ext)) {
        warnings.push({
          file: relPath,
          line: 0,
          category: "Unexpected file type",
          raw: `File type ${ext} is unexpected in a skill`,
        });
        continue;
      }

      const bunFile = Bun.file(filePath);
      const fileSize = bunFile.size;

      // File size check
      if (fileSize > MAX_FILE_SIZE) {
        warnings.push({
          file: relPath,
          line: 0,
          category: "Large file",
          raw: `File size ${fileSize} bytes exceeds ${MAX_FILE_SIZE} bytes`,
        });
      }

      // Read file bytes for binary detection
      const bytes = new Uint8Array(await bunFile.arrayBuffer());
      const binaryType = hasBinaryMagic(bytes);
      if (binaryType) {
        warnings.push({
          file: relPath,
          line: 0,
          category: "Binary file",
          raw: `Detected ${binaryType} signature`,
        });
        continue;
      }

      // Decode as UTF-8 for text analysis
      let content: string;
      try {
        content = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      } catch {
        // Not valid UTF-8 — treat as binary
        warnings.push({
          file: relPath,
          line: 0,
          category: "Binary file",
          raw: "File contains invalid UTF-8 sequences",
        });
        continue;
      }

      // Minified JS/TS check
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

      // Run all pattern detectors
      const detectors = [
        detectInvisibleUnicode,
        detectHiddenHtmlCss,
        detectMarkdownHiding,
        detectObfuscation,
        detectSuspiciousUrls,
        detectDangerousPatterns,
        detectTagInjection,
      ];

      for (const detect of detectors) {
        const matches = detect(content);
        for (const m of matches) {
          warnings.push({ file: relPath, ...m });
        }
      }
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
