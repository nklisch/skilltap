/// <reference path="../anti-trojan-source.d.ts" />
import { hasConfusables } from "anti-trojan-source";
import { replace as removeInvisible } from "out-of-character";

export type PatternMatch = {
  line: number | [number, number];
  category: string;
  raw: string;
  visible?: string;
  decoded?: string;
};

// Escape non-printing chars for display (e.g., U+200B → [U+200B])
function escapeInvisible(text: string): string {
  return [...text]
    .map((char) => {
      // biome-ignore lint/style/noNonNullAssertion: char is always a single code point
      const cp = char.codePointAt(0)!;
      if (cp >= 0x20 && cp <= 0x7e) return char;
      if (cp === 0x09 || cp === 0x0a || cp === 0x0d) return char;
      return `[U+${cp.toString(16).toUpperCase().padStart(4, "0")}]`;
    })
    .join("");
}

export function detectInvisibleUnicode(content: string): PatternMatch[] {
  const findings = hasConfusables({
    sourceText: content,
    detailed: true,
  }) as Array<{
    line: number;
    column: number;
    codePoint: string;
    name: string;
    category: string;
    snippet: string;
  }>;
  if (!findings || findings.length === 0) return [];

  const lines = content.split("\n");
  const lineNums = new Set<number>();
  for (const f of findings) lineNums.add(f.line);

  return Array.from(lineNums)
    .sort((a, b) => a - b)
    .map((lineNum) => {
      const lineContent = lines[lineNum - 1] ?? "";
      return {
        line: lineNum,
        category: "Invisible Unicode",
        raw: escapeInvisible(lineContent),
        visible: removeInvisible(lineContent),
      };
    });
}

export function detectHiddenHtmlCss(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");

  // Multi-line HTML comments
  const htmlCommentRe = /<!--[\s\S]*?-->/g;
  for (const m of content.matchAll(htmlCommentRe)) {
    const before = content.slice(0, m.index);
    const startLine = before.split("\n").length;
    const matchLineCount = m[0].split("\n").length;
    const line: number | [number, number] =
      matchLineCount === 1
        ? startLine
        : [startLine, startLine + matchLineCount - 1];
    const raw = m[0].length > 100 ? `${m[0].slice(0, 100)}...` : m[0];
    results.push({ line, category: "HTML comment", raw });
  }

  // Inline CSS hiding patterns
  const cssHidingRe =
    /(?:display\s*:\s*none|opacity\s*:\s*0(?:\s*;|\s*$)|font-size\s*:\s*0(?:px)?(?:\s*;|\s*$)|visibility\s*:\s*hidden|position\s*:\s*absolute[^;]*left\s*:\s*-\d+)/i;
  for (const [i, line] of lines.entries()) {
    if (cssHidingRe.test(line)) {
      results.push({ line: i + 1, category: "Hidden CSS", raw: line });
    }
  }

  return results;
}

export function detectMarkdownHiding(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");

  // Markdown hidden reference/comment syntax
  const hiddenRefRe = /^\s*\[(?:\/\/|comment|[^\]]+)\]:\s*#\s*\(/i;
  // Image alt text with suspicious instructions
  const suspiciousAltRe =
    /!\[([^\]]*(?:ignore|system\s*prompt|previous\s*instructions?|disregard|forget)[^\]]*)\]/i;

  for (const [i, line] of lines.entries()) {
    if (hiddenRefRe.test(line)) {
      results.push({
        line: i + 1,
        category: "Markdown comment",
        raw: line.trim(),
      });
    } else if (suspiciousAltRe.test(line)) {
      results.push({
        line: i + 1,
        category: "Markdown comment",
        raw: line.trim(),
      });
    }
  }

  return results;
}

// Check if a string decodes from base64 to mostly printable ASCII
function decodesToPrintable(s: string): boolean {
  try {
    const text = Buffer.from(s, "base64").toString("utf8");
    let printable = 0;
    for (const ch of text) {
      const cp = ch.codePointAt(0) ?? 0;
      if (
        (cp >= 0x20 && cp <= 0x7e) ||
        cp === 0x09 ||
        cp === 0x0a ||
        cp === 0x0d
      )
        printable++;
    }
    return printable / text.length > 0.7;
  } catch {
    return false;
  }
}

// Heuristic: does a short string (10-19 chars) look like base64 vs an English word?
function looksLikeBase64(s: string): boolean {
  // + char doesn't appear in English words or paths (/ is too ambiguous)
  if (/\+/.test(s)) return true;
  // Digits are rare in 10+ char English words
  if (/\d/.test(s)) return true;
  // Non-CamelCase mixed case — base64 has random case transitions
  // But only flag if the decoded content is plausible text (filters out
  // brand names like PostgreSQL that decode to gibberish)
  if (
    /[a-z][A-Z]/.test(s) &&
    !/^[a-zA-Z][a-z]+(?:[A-Z][a-z]+)*$/.test(s) &&
    decodesToPrintable(s)
  )
    return true;
  return false;
}

export function detectObfuscation(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");

  // Base64 detection — two-tier approach:
  //   20+ base chars: flag unconditionally
  //   10-19 base chars: flag only if padded (=) or has base64-specific traits
  const base64Re = /[A-Za-z0-9+/]{10,}={0,3}/g;
  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(base64Re.source, "g");
    for (const m of line.matchAll(lineRe)) {
      // Skip if preceded by URL separators (likely part of a URL path)
      const before = line.slice(Math.max(0, m.index - 4), m.index);
      if (/[/.]/.test(before)) continue;
      const base = m[0].replace(/=+$/, "");
      const hasPadding = base.length < m[0].length;
      // Short matches need extra confirmation to avoid flagging English words
      if (base.length < 20 && !hasPadding && !looksLikeBase64(base)) continue;
      // Letter-only sequences (with optional slashes) are identifiers or words, not base64.
      // Real base64 of 10+ chars statistically always contains digits or + characters.
      // P(all-letters in 20 random base64 chars) ≈ 1.5%, at 30 chars ≈ 0.18%.
      if (/^[a-zA-Z/]+$/.test(base)) continue;
      let decoded: string | undefined;
      try {
        const bytes = Buffer.from(m[0], "base64");
        const text = bytes.toString("utf8");
        decoded = text.length > 80 ? `${text.slice(0, 80)}...` : text;
      } catch {
        // not valid base64
      }
      results.push({
        line: i + 1,
        category: "Base64 block",
        raw: m[0],
        decoded,
      });
    }
  }

  // Hex-encoded strings: 3+ consecutive \xNN sequences
  const hexRe = /((?:\\x[0-9a-fA-F]{2}){3,})/g;
  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(hexRe.source, "g");
    for (const m of line.matchAll(lineRe)) {
      let decoded: string | undefined;
      try {
        decoded = m[1]?.replace(/\\x([0-9a-fA-F]{2})/g, (_, h) =>
          String.fromCharCode(parseInt(h, 16)),
        );
      } catch {
        // ignore
      }
      results.push({
        line: i + 1,
        category: "Hex encoding",
        // biome-ignore lint/style/noNonNullAssertion: m[1] is always defined — the outer group always captures
        raw: m[1]!,
        decoded,
      });
    }
  }

  // Variable expansion to hide commands: `c${"ur"+"l"}` style
  const varExpansionRe = /\$\{["'][^"']+["']\s*\+\s*["'][^"']*["']\}/g;
  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(varExpansionRe.source, "g");
    for (const m of line.matchAll(lineRe)) {
      results.push({
        line: i + 1,
        category: "Variable expansion",
        raw: m[0],
      });
    }
  }

  // Data URIs
  const dataUriRe =
    /data:[a-zA-Z]+\/[a-zA-Z0-9+\-.]+(?:;[^,]+)?(?:,|;base64,)/g;
  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(dataUriRe.source, "g");
    for (const m of line.matchAll(lineRe)) {
      results.push({ line: i + 1, category: "Data URI", raw: m[0] });
    }
  }

  return results;
}

const SUSPICIOUS_DOMAINS = [
  "ngrok.io",
  "webhook.site",
  "requestbin.com",
  "pipedream.com",
  "burpcollaborator.net",
  "interact.sh",
  "canarytokens.com",
  "hookbin.com",
  "beeceptor.com",
];

export function detectSuspiciousUrls(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");
  const urlRe = /https?:\/\/[^\s)\]'"<>]+/g;

  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(urlRe.source, "g");
    for (const m of line.matchAll(lineRe)) {
      const url = m[0];
      const isSuspiciousDomain = SUSPICIOUS_DOMAINS.some((d) =>
        url.includes(d),
      );
      const isLocalhost =
        /^https?:\/\/(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])/.test(url);
      const hasInterpolation =
        !isLocalhost && /\$\{|\$\(|\{\{/.test(url);
      const hasSuspiciousParams = /[?&](?:data|exfil|d|payload)=/.test(url);

      if (isSuspiciousDomain || hasInterpolation || hasSuspiciousParams) {
        results.push({ line: i + 1, category: "Suspicious URL", raw: url });
      }
    }
  }

  return results;
}

export function detectDangerousPatterns(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");

  // Shell commands — word boundary, not inside a longer word
  const shellCmdRe =
    /\b(curl|wget|eval|exec)\b(?:\s|$|\()|\bsh\s+-c\b|\bbash\s+-c\b/;
  // Sensitive file paths
  const sensitivePathRe =
    /~\/\.(?:ssh|aws|gnupg|config)\/|\/etc\/(?:passwd|shadow|hosts)\b/;
  // Credential/environment variable access
  const credentialRe =
    /\$(?:SSH_(?:KEY|AUTH_SOCK|PRIVATE_KEY)|AWS_(?:SECRET|ACCESS_KEY(?:_ID)?)|GITHUB_TOKEN|API_KEY|PASSWORD\b|TOKEN\b)|process\.env\.(?:API_KEY|SECRET|TOKEN|PASSWORD|PRIVATE_KEY|CREDENTIAL|AWS_|SSH_|GITHUB_TOKEN)/;

  for (const [i, line] of lines.entries()) {
    if (shellCmdRe.test(line)) {
      results.push({
        line: i + 1,
        category: "Shell command",
        raw: line.trim(),
      });
    }

    const pathMatch = sensitivePathRe.exec(line);
    if (pathMatch) {
      results.push({
        line: i + 1,
        category: "Sensitive path",
        raw: pathMatch[0],
      });
    }

    const credMatch = credentialRe.exec(line);
    if (credMatch) {
      results.push({
        line: i + 1,
        category: "Credential access",
        raw: credMatch[0],
      });
    }
  }

  return results;
}

export function detectTagInjection(content: string): PatternMatch[] {
  const results: PatternMatch[] = [];
  const lines = content.split("\n");
  const tagRe =
    /<\/(system|instructions|context|tool_response|untrusted(?:-content)?[^>]*?)>/gi;

  for (const [i, line] of lines.entries()) {
    const lineRe = new RegExp(tagRe.source, "gi");
    for (const m of line.matchAll(lineRe)) {
      results.push({ line: i + 1, category: "Tag injection", raw: m[0] });
    }
  }

  return results;
}
