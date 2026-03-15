import type { Config } from "../schemas/config";
import type { Result } from "../types";
import { err, ok, ScanError } from "../types";
import {
  claudeAdapter,
  codexAdapter,
  geminiAdapter,
  opencodeAdapter,
} from "./adapters";
import { createCustomAdapter } from "./custom";
import { createOllamaAdapter } from "./ollama";
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

/** All valid values for security.agent in config. */
export const KNOWN_AGENT_NAMES: readonly string[] = [...Object.keys(ADAPTER_MAP), "ollama"];

/** Verify an adapter is reachable on PATH. Returns ok(adapter) or err with install hint. */
async function verifyAdapterAvailable(
  adapter: AgentAdapter,
  configuredName: string,
): Promise<Result<AgentAdapter, ScanError>> {
  const available = await adapter.detect();
  if (!available) {
    return err(
      new ScanError(
        `Configured agent '${configuredName}' not found on PATH.`,
        `Install ${adapter.name} or change security.agent_cli in config.toml`,
      ),
    );
  }
  return ok(adapter);
}

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
  const agentSetting = config.security.agent_cli;

  // 1. Known adapter name
  if (agentSetting && !agentSetting.startsWith("/")) {
    if (agentSetting === "ollama") {
      return verifyAdapterAvailable(
        createOllamaAdapter(config.security.ollama_model),
        "ollama",
      );
    }

    const adapter = ADAPTER_MAP[agentSetting];
    if (!adapter) {
      return err(
        new ScanError(
          `Unknown agent '${agentSetting}' in config.`,
          `Valid agents: ${KNOWN_AGENT_NAMES.join(", ")}, or an absolute path`,
        ),
      );
    }

    return verifyAdapterAvailable(adapter, agentSetting);
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
