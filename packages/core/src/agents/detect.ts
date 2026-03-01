import type { Config } from "../schemas/config";
import type { Result } from "../types";
import { err, ok, ScanError } from "../types";
import { claudeAdapter } from "./claude";
import { codexAdapter } from "./codex";
import { createCustomAdapter } from "./custom";
import { geminiAdapter } from "./gemini";
import { createOllamaAdapter } from "./ollama";
import { opencodeAdapter } from "./opencode";
import type { AgentAdapter } from "./types";

/** All known adapters. Ollama uses empty model — resolved at invocation time. */
const ALL_ADAPTERS: AgentAdapter[] = [
  claudeAdapter,
  geminiAdapter,
  codexAdapter,
  opencodeAdapter,
  createOllamaAdapter(""),
];

/** Map of cliName → adapter constructor/instance for config lookup. */
const ADAPTER_MAP: Record<string, AgentAdapter> = {
  claude: claudeAdapter,
  gemini: geminiAdapter,
  codex: codexAdapter,
  opencode: opencodeAdapter,
};

/** Detect which agent CLIs are available on PATH. */
export async function detectAgents(): Promise<AgentAdapter[]> {
  const results = await Promise.all(
    ALL_ADAPTERS.map(async (adapter) => ({
      adapter,
      available: await adapter.detect(),
    })),
  );
  return results.filter((r) => r.available).map((r) => r.adapter);
}

/**
 * Resolve which agent to use for semantic scanning.
 *
 * Priority:
 * 1. config.security.agent set to known name → find matching adapter, verify detect()
 * 2. config.security.agent is absolute path → createCustomAdapter(path)
 * 3. Empty → detectAgents() → if none found, return ok(null) → if found and onSelectAgent provided, call it
 */
export async function resolveAgent(
  config: Config,
  onSelectAgent?: (detected: AgentAdapter[]) => Promise<AgentAdapter | null>,
): Promise<Result<AgentAdapter | null, ScanError>> {
  const agentSetting = config.security.agent;

  // 1. Known adapter name
  if (agentSetting && !agentSetting.startsWith("/")) {
    // Check for ollama specially
    if (agentSetting === "ollama") {
      const adapter = createOllamaAdapter(config.security.ollama_model);
      const available = await adapter.detect();
      if (!available) {
        return err(
          new ScanError(
            `Configured agent 'ollama' not found on PATH.`,
            "Install Ollama or change security.agent in config.toml",
          ),
        );
      }
      return ok(adapter);
    }

    const adapter = ADAPTER_MAP[agentSetting];
    if (!adapter) {
      return err(
        new ScanError(
          `Unknown agent '${agentSetting}' in config.`,
          "Valid agents: claude, gemini, codex, opencode, ollama, or an absolute path",
        ),
      );
    }

    const available = await adapter.detect();
    if (!available) {
      return err(
        new ScanError(
          `Configured agent '${agentSetting}' not found on PATH.`,
          `Install ${adapter.name} or change security.agent in config.toml`,
        ),
      );
    }
    return ok(adapter);
  }

  // 2. Absolute path
  if (agentSetting?.startsWith("/")) {
    const adapter = createCustomAdapter(agentSetting);
    return ok(adapter);
  }

  // 3. Auto-detect
  const detected = await detectAgents();
  if (detected.length === 0) return ok(null);

  if (onSelectAgent) {
    const chosen = await onSelectAgent(detected);
    return ok(chosen);
  }

  // Default to first detected
  // biome-ignore lint/style/noNonNullAssertion: length > 0 checked above
  return ok(detected[0]!);
}
