# Pattern: AgentAdapter Strategy

Agent CLI tools (Claude, Gemini, Codex, etc.) are wrapped as `AgentAdapter` objects with a shared interface, resolved via a registry with priority ordering.

## Rationale

Semantic scanning needs to invoke different AI CLIs. Each has different binary names, invocation syntax, and output formats. The `AgentAdapter` interface normalizes these behind `detect()` + `invoke()`, with factory functions to avoid repetition across similar adapters. Resolution follows config → absolute path → auto-detect priority, with an optional callback for user selection.

## Examples

### Example 1: Interface definition
**File**: `packages/core/src/agents/types.ts:4`
```typescript
export interface AgentAdapter {
  readonly name: string;
  readonly cliName: string;
  detect(): Promise<boolean>;
  invoke(prompt: string): Promise<Result<AgentResponse, ScanError>>;
}
```

### Example 2: Factory for CLI-based adapters
**File**: `packages/core/src/agents/factory.ts:8`
```typescript
export function createCliAdapter(
  name: string,
  cliName: string,
  buildCommand: InvokeCommand,
): AgentAdapter {
  return {
    name,
    cliName,
    async detect() {
      try {
        await $`which ${cliName}`.quiet();
        return true;
      } catch {
        return false;
      }
    },
    async invoke(prompt) {
      try {
        const result = await buildCommand(prompt);
        const raw = result.stdout.toString().trim();
        const parsed = extractAgentResponse(raw);
        if (!parsed)
          return ok({ score: 0, reason: "Could not parse agent response" });
        return ok(parsed);
      } catch (e) {
        return err(new ScanError(`${name} invocation failed: ${e instanceof Error ? e.message : String(e)}`));
      }
    },
  };
}
```

### Example 3: Concrete adapter using the factory
**File**: `packages/core/src/agents/adapters.ts:4`
```typescript
export const claudeAdapter = createCliAdapter(
  "Claude Code",
  "claude",
  (prompt) => $`claude --print ${prompt}`.quiet(),
);
```

### Example 4: Priority-based resolution
**File**: `packages/core/src/agents/detect.ts:50`
```typescript
export async function resolveAgent(
  config: Config,
  onSelectAgent?: (detected: AgentAdapter[]) => Promise<AgentAdapter | null>,
): Promise<Result<AgentAdapter | null, ScanError>> {
  const agentSetting = config.security.agent_cli;

  // 1. Known adapter name in config
  if (agentSetting && !agentSetting.startsWith("/")) { /* lookup + detect */ }

  // 2. Absolute path → custom adapter
  if (agentSetting?.startsWith("/")) { return ok(createCustomAdapter(agentSetting)); }

  // 3. Auto-detect → callback for user selection
  const detected = await detectAgents();
  if (detected.length === 0) return ok(null);
  if (onSelectAgent) return ok(await onSelectAgent(detected));
  return ok(detected[0]!);
}
```

## When to Use

- Adding a new agent CLI (e.g., Aider, Windsurf) — create an adapter with `createCliAdapter()`
- Adding non-CLI agents (e.g., Ollama API) — implement `AgentAdapter` directly with `createOllamaAdapter()` factory
- Resolving agent at runtime — always go through `resolveAgent()`, never instantiate adapters directly in commands

## When NOT to Use

- `SourceAdapter` is a separate pattern for resolving install sources — don't conflate the two
- Don't add agent-specific logic to `resolveAgent()` — keep it in the adapter's `invoke()` method

## Common Violations

- Implementing adapters as classes instead of factory-created object literals
- Throwing from `invoke()` instead of returning `Result<>` — errors must be wrapped
- Failing closed on parse errors — agent invocation uses fail-open (score 0) when response can't be parsed
- Hardcoding agent names in commands instead of going through the registry
