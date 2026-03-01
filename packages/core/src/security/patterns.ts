import { hasConfusables } from "anti-trojan-source"
import { replace as removeInvisible } from "out-of-character"

export type PatternMatch = {
  line: number | [number, number]
  category: string
  raw: string
  visible?: string
  decoded?: string
}

// Escape non-printing chars for display (e.g., U+200B → [U+200B])
function escapeInvisible(text: string): string {
  return [...text]
    .map((char) => {
      const cp = char.codePointAt(0)!
      if (cp >= 0x20 && cp <= 0x7e) return char
      if (cp === 0x09 || cp === 0x0a || cp === 0x0d) return char
      return `[U+${cp.toString(16).toUpperCase().padStart(4, "0")}]`
    })
    .join("")
}

export function detectInvisibleUnicode(content: string): PatternMatch[] {
  const findings = hasConfusables({ sourceText: content, detailed: true }) as Array<{
    line: number
    column: number
    codePoint: string
    name: string
    category: string
    snippet: string
  }>
  if (!findings || findings.length === 0) return []

  const lines = content.split("\n")
  const lineNums = new Set<number>()
  for (const f of findings) lineNums.add(f.line)

  return Array.from(lineNums)
    .sort((a, b) => a - b)
    .map((lineNum) => {
      const lineContent = lines[lineNum - 1] ?? ""
      return {
        line: lineNum,
        category: "Invisible Unicode",
        raw: escapeInvisible(lineContent),
        visible: removeInvisible(lineContent),
      }
    })
}

export function detectHiddenHtmlCss(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")

  // Multi-line HTML comments
  const htmlCommentRe = /<!--[\s\S]*?-->/g
  let m: RegExpExecArray | null
  while ((m = htmlCommentRe.exec(content)) !== null) {
    const before = content.slice(0, m.index)
    const startLine = before.split("\n").length
    const matchLineCount = m[0].split("\n").length
    const line: number | [number, number] =
      matchLineCount === 1 ? startLine : [startLine, startLine + matchLineCount - 1]
    const raw = m[0].length > 100 ? m[0].slice(0, 100) + "..." : m[0]
    results.push({ line, category: "HTML comment", raw })
  }

  // Inline CSS hiding patterns
  const cssHidingRe =
    /(?:display\s*:\s*none|opacity\s*:\s*0(?:\s*;|\s*$)|font-size\s*:\s*0(?:px)?(?:\s*;|\s*$)|visibility\s*:\s*hidden|position\s*:\s*absolute[^;]*left\s*:\s*-\d+)/i
  for (let i = 0; i < lines.length; i++) {
    if (cssHidingRe.test(lines[i]!)) {
      results.push({ line: i + 1, category: "Hidden CSS", raw: lines[i]! })
    }
  }

  return results
}

export function detectMarkdownHiding(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")

  // Markdown hidden reference/comment syntax
  const hiddenRefRe = /^\s*\[(?:\/\/|comment|[^\]]+)\]:\s*#\s*\(/i
  // Image alt text with suspicious instructions
  const suspiciousAltRe =
    /!\[([^\]]*(?:ignore|system\s*prompt|previous\s*instructions?|disregard|forget)[^\]]*)\]/i

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    if (hiddenRefRe.test(line)) {
      results.push({ line: i + 1, category: "Markdown comment", raw: line.trim() })
    } else if (suspiciousAltRe.test(line)) {
      results.push({ line: i + 1, category: "Markdown comment", raw: line.trim() })
    }
  }

  return results
}

export function detectObfuscation(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")

  // Base64 blocks: 60+ consecutive base64 chars not part of a URL
  const base64Re = /[A-Za-z0-9+/]{60,}={0,3}/g
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(base64Re.source, "g")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      // Skip if preceded by URL separators (likely part of a URL path)
      const before = line.slice(Math.max(0, m.index - 4), m.index)
      if (/[/.]/.test(before)) continue
      let decoded: string | undefined
      try {
        const bytes = Buffer.from(m[0], "base64")
        const text = bytes.toString("utf8")
        // Only treat as suspicious if it decodes to plausible text or binary
        decoded = text.length > 80 ? text.slice(0, 80) + "..." : text
      } catch {
        // not valid base64
      }
      results.push({ line: i + 1, category: "Base64 block", raw: m[0], decoded })
    }
  }

  // Hex-encoded strings: 3+ consecutive \xNN sequences
  const hexRe = /((?:\\x[0-9a-fA-F]{2}){3,})/g
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(hexRe.source, "g")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      let decoded: string | undefined
      try {
        decoded = m[1]!.replace(/\\x([0-9a-fA-F]{2})/g, (_, h) =>
          String.fromCharCode(parseInt(h, 16)),
        )
      } catch {
        // ignore
      }
      results.push({ line: i + 1, category: "Hex encoding", raw: m[1]!, decoded })
    }
  }

  // Variable expansion to hide commands: `c${"ur"+"l"}` style
  const varExpansionRe = /\$\{["'][^"']+["']\s*\+\s*["'][^"']*["']\}/g
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(varExpansionRe.source, "g")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      results.push({ line: i + 1, category: "Variable expansion", raw: m[0] })
    }
  }

  // Data URIs
  const dataUriRe = /data:[a-zA-Z]+\/[a-zA-Z0-9+\-.]+(?:;[^,]+)?(?:,|;base64,)/g
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(dataUriRe.source, "g")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      results.push({ line: i + 1, category: "Data URI", raw: m[0] })
    }
  }

  return results
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
]

export function detectSuspiciousUrls(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")
  const urlRe = /https?:\/\/[^\s)\]'"<>]+/g

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(urlRe.source, "g")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      const url = m[0]!
      const isSuspiciousDomain = SUSPICIOUS_DOMAINS.some((d) => url.includes(d))
      const hasInterpolation = /\$\{|\$\(|\{\{/.test(url)
      const hasSuspiciousParams = /[?&](?:data|exfil|d|payload)=/.test(url)

      if (isSuspiciousDomain || hasInterpolation || hasSuspiciousParams) {
        results.push({ line: i + 1, category: "Suspicious URL", raw: url })
      }
    }
  }

  return results
}

export function detectDangerousPatterns(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")

  // Shell commands — word boundary, not inside a longer word
  const shellCmdRe = /\b(curl|wget|eval|exec)\b(?:\s|$|\()|\bsh\s+-c\b|\bbash\s+-c\b/
  // Sensitive file paths
  const sensitivePathRe =
    /~\/\.(?:ssh|aws|gnupg|config)\/|\/etc\/(?:passwd|shadow|hosts)\b/
  // Credential/environment variable access
  const credentialRe =
    /\$(?:SSH_(?:KEY|AUTH_SOCK|PRIVATE_KEY)|AWS_(?:SECRET|ACCESS_KEY(?:_ID)?)|GITHUB_TOKEN|API_KEY|PASSWORD\b|TOKEN\b)|process\.env\./

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!

    if (shellCmdRe.test(line)) {
      results.push({ line: i + 1, category: "Shell command", raw: line.trim() })
    }

    const pathMatch = sensitivePathRe.exec(line)
    if (pathMatch) {
      results.push({ line: i + 1, category: "Sensitive path", raw: pathMatch[0] })
    }

    const credMatch = credentialRe.exec(line)
    if (credMatch) {
      results.push({ line: i + 1, category: "Credential access", raw: credMatch[0] })
    }
  }

  return results
}

export function detectTagInjection(content: string): PatternMatch[] {
  const results: PatternMatch[] = []
  const lines = content.split("\n")
  const tagRe = /<\/(system|instructions|context|tool_response|untrusted(?:-content)?[^>]*?)>/gi

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const lineRe = new RegExp(tagRe.source, "gi")
    let m: RegExpExecArray | null
    while ((m = lineRe.exec(line)) !== null) {
      results.push({ line: i + 1, category: "Tag injection", raw: m[0] })
    }
  }

  return results
}
