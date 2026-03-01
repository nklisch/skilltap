import { parse, stringify } from "smol-toml"
import { mkdir } from "node:fs/promises"
import { join } from "node:path"
import { homedir } from "node:os"
import { z } from "zod/v4"
import { ok, err, UserError, type Result } from "./types"
import { ConfigSchema, type Config } from "./schemas/config"
import { InstalledJsonSchema, type InstalledJson } from "./schemas/installed"

function configDir(): string {
  const xdg = process.env.XDG_CONFIG_HOME
  return xdg ? join(xdg, "skilltap") : join(homedir(), ".config", "skilltap")
}

export async function ensureDirs(): Promise<Result<void>> {
  const dir = configDir()
  try {
    await mkdir(join(dir, "taps"), { recursive: true })
    await mkdir(join(dir, "cache"), { recursive: true })
    return ok(undefined)
  } catch (e) {
    return err(new UserError(`Failed to create config directories: ${e}`))
  }
}

// Static template preserves comments for user reference.
// smol-toml.stringify() strips comments, so saveConfig() will lose them — acceptable.
const DEFAULT_CONFIG_TEMPLATE = `# Default settings for install commands
[defaults]
# Agent-specific directories to also symlink to on every install
# Valid values: "claude-code", "cursor", "codex", "gemini", "windsurf"
also = []

# Auto-accept prompts (same as --yes). Auto-selects all skills and
# auto-accepts clean installs. Security warnings still require confirmation.
# Scope still prompts unless a default scope is also set.
yes = false

# Default install scope. If set, skips the scope prompt.
# Values: "global", "project", or "" (prompt)
scope = ""

# Security scanning settings
[security]
# Scan mode: "static" (Layer 1 only), "semantic" (Layer 1 + Layer 2), "off"
scan = "static"

# What to do when security warnings are found:
#   "prompt" = show warnings and ask user (default)
#   "fail"   = abort immediately, no prompt (same as --strict)
on_warn = "prompt"

# Prevent --skip-scan from being used. When true, security scanning
# cannot be bypassed via CLI flags. Useful for org/machine-level policy.
require_scan = false

# Agent CLI to use for semantic scanning.
# Values: "claude", "gemini", "codex", "opencode", "ollama", or an absolute path
# to a custom binary (e.g. "/usr/local/bin/my-llm").
# Empty string = prompt on first use, then save selection.
agent = ""

# Risk threshold for semantic scan (0-10, chunks scoring >= this are flagged)
threshold = 5

# Max total skill directory size in bytes before warning (default 50KB)
max_size = 51200

# Ollama model for semantic scanning (if using ollama adapter)
ollama_model = ""

# Agent mode — for when skilltap is invoked by an AI agent, not a human.
["agent-mode"]
# Enable agent mode. When true, all prompts auto-accept or hard-fail.
enabled = false

# Default scope for agent installs. Required when agent mode is enabled.
# Values: "global", "project"
scope = "project"

# Tap definitions (repeatable section)
# [[taps]]
# name = "home"
# url = "https://gitea.example.com/nathan/my-skills-tap"
`

export async function loadConfig(): Promise<Result<Config>> {
  const dir = configDir()
  const file = join(dir, "config.toml")

  const dirsResult = await ensureDirs()
  if (!dirsResult.ok) return dirsResult

  const f = Bun.file(file)
  const exists = await f.exists()

  if (!exists) {
    try {
      await Bun.write(file, DEFAULT_CONFIG_TEMPLATE)
    } catch (e) {
      return err(new UserError(`Failed to write default config: ${e}`))
    }
    return ok(ConfigSchema.parse({}))
  }

  let text: string
  try {
    text = await f.text()
  } catch (e) {
    return err(new UserError(`Failed to read config.toml: ${e}`))
  }

  let raw: unknown
  try {
    raw = parse(text)
  } catch (e) {
    return err(new UserError(`Invalid TOML in config.toml: ${e}`))
  }

  const result = ConfigSchema.safeParse(raw)
  if (!result.success) {
    return err(new UserError(`Invalid config.toml: ${z.prettifyError(result.error)}`))
  }

  return ok(result.data)
}

export async function saveConfig(config: Config): Promise<Result<void>> {
  const dir = configDir()
  const file = join(dir, "config.toml")

  const dirsResult = await ensureDirs()
  if (!dirsResult.ok) return dirsResult

  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const text = stringify(config as any)
    await Bun.write(file, text)
    return ok(undefined)
  } catch (e) {
    return err(new UserError(`Failed to save config: ${e}`))
  }
}

const DEFAULT_INSTALLED: InstalledJson = { version: 1, skills: [] }

export async function loadInstalled(): Promise<Result<InstalledJson>> {
  const dir = configDir()
  const file = join(dir, "installed.json")

  const f = Bun.file(file)
  const exists = await f.exists()

  if (!exists) {
    return ok(DEFAULT_INSTALLED)
  }

  let raw: unknown
  try {
    raw = await f.json()
  } catch (e) {
    return err(new UserError(`Invalid JSON in installed.json: ${e}`))
  }

  const result = InstalledJsonSchema.safeParse(raw)
  if (!result.success) {
    return err(new UserError(`Invalid installed.json: ${z.prettifyError(result.error)}`))
  }

  return ok(result.data)
}

export async function saveInstalled(installed: InstalledJson): Promise<Result<void>> {
  const dir = configDir()
  const file = join(dir, "installed.json")

  const dirsResult = await ensureDirs()
  if (!dirsResult.ok) return dirsResult

  try {
    await Bun.write(file, JSON.stringify(installed, null, 2))
    return ok(undefined)
  } catch (e) {
    return err(new UserError(`Failed to save installed.json: ${e}`))
  }
}
